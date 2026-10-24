# Architecture

A64 Studio is deliberately split into two languages and two Android processes.

```
┌──────────────────────────────────────────────────────────────┐
│ app process (dev.a64studio)                                  │
│                                                              │
│  Jetpack Compose UI  ──►  StudioViewModel (StateFlow)        │
│        │                        │                            │
│        │                        ├─ ProjectStore (.s files)   │
│        │                        ├─ CoreApi ──► JNI            │
│        │                        └─ WorkerClient ──► Messenger │
│        │                                                      │
│        └─ editor, registers, machine code, memory, CPU lab,   │
│           explorer, hex patcher, benchmarks, settings         │
└───────────────────────────┬──────────────────────────────────┘
                            │ Binder / Messenger
┌───────────────────────────▼──────────────────────────────────┐
│ :worker process (AssemblerService)                           │
│                                                              │
│  single execution thread                                     │
│  watchdog Handler ──► Process.killProcess on timeout         │
│  liba64core.so: nativeRun / nativeRunHex / nativeBenchmark   │
└───────────────────────────┬──────────────────────────────────┘
                            │ JNI (JSON strings)
┌───────────────────────────▼──────────────────────────────────┐
│ Rust core (liba64core.so, arm64-v8a)                         │
│                                                              │
│  asm/       two-pass assembler, sections, relocations        │
│  lexer.rs   UTF-16 token stream for Compose                  │
│  disasm.rs  yaxpeax-arm decoder + hex dump                   │
│  exec.rs    mmap / W^X / I-cache flush / trampoline          │
│  cpu.rs     real CPU and HWCAP detection                     │
│  bench.rs   guest kernels and host assembler benchmark       │
│  syntax.rs  instruction reference / vocabulary               │
└──────────────────────────────────────────────────────────────┘
```

## Why two processes?

Guest assembly is arbitrary native code. It can crash, corrupt its own
registers, overwrite its stack, or loop forever. Running it in the editor
process would risk the project file and the UI. The `:worker` process gives a
hard boundary:

- The editor process only sends source/hex and receives a JSON result.
- The worker is started with `bindService` and stopped after each request.
- A `Handler` watchdog in the worker kills the worker process if the guest
  exceeds the deadline. It writes `cacheDir/worker_timeout_<id>` first, so the
  client can distinguish a timeout from an ordinary crash.
- The client has its own deadline (`timeout + 1.5 s`) and treats a dead
  binding, a dead service, or a missing reply as a failure.

This is process isolation, not a security sandbox. The worker runs with the
same Android UID and sandbox as the app. See
[LIMITATIONS.md](LIMITATIONS.md).

## JNI surface

`NativeBridge` has a small number of string-returning methods. Every method
returns JSON; the Kotlin layer parses it with `org.json`. This keeps the ABI
stable and avoids generated headers.

| Kotlin method | Rust entry point | Result |
|---|---|---|
| `nativeVersion()` | `nativeVersion` | version/name |
| `nativeAssemble(source)` | `nativeAssemble` | `AssembleResult` |
| `nativeLex(text)` | `nativeLex` | line count + token stream |
| `nativeDisassemble(hex, base)` | `nativeDisassemble` | instruction list |
| `nativeDisassembleBytes(bytes, base)` | `nativeDisassembleBytes` | instruction list |
| `nativeRun(source, timeoutMs)` | `nativeRun` | `ExecResult` |
| `nativeRunHex(hex, entry, timeoutMs)` | `nativeRunHex` | `ExecResult` |
| `nativeCpuInfo()` | `nativeCpuInfo` | CPU model, HWCAP/HWCAP2 |
| `nativeReference()` | `nativeReference` | explorer/autocomplete data |
| `nativeBenchmarkList()` | `nativeBenchmarkList` | benchmark definitions |
| `nativeRunBenchmark(id, timeoutMs)` | `nativeRunBenchmark` | benchmark result |
| `nativeHexDump(bytes, base)` | `nativeHexDump` | formatted hex dump |

`CoreApi` wraps the JSON in typed Kotlin models and turns failures into
`Result`. It never invents a result when the native library is missing; it
returns an error that the UI displays.

## Assembler

`core/src/asm/` is a two-pass, section-aware assembler:

1. **Lex/parse** - lines, labels, directives, operands, comments. A
   `HighlightSession` can reuse the lexer for the editor.
2. **Pass 1** - collect labels, section sizes and statement sizes. The image
   layout is fixed: `.text` starts at offset 0, `.data` starts at the next page
   boundary when non-empty.
3. **Pass 2** - encode instructions, resolve local labels, apply relocations,
   emit diagnostics with 1-based line/column and UTF-16 column offsets.

Supported sections/directives include `.text`, `.data`, `.bss`, `.global`,
`.section`, `.align`, `.balign`, `.byte`, `.hword`, `.word`, `.quad`, `.asciz`,
`.string`, `.space`, `.skip`, `.equ`, `.set`, `.type`, `.size`, `.ltorg` and
literal pools. Unknown directives and unsupported operand forms are errors,
not silent guesses.

The encoder is hand-written rather than a wrapper around LLVM. The
`scripts/verify-encodings.py` cross-check assembles 270 instruction forms with
the NDK's LLVM and compares the bytes with the Rust encoder. This keeps the
"what you see is what runs" property: the Instruction Explorer is generated
from the same supported-instruction table.

## Execution image

For a source run:

1. Assemble `.text` and `.data`.
2. Allocate one anonymous mapping with `PROT_READ | PROT_WRITE`.
3. Copy `.text` at offset 0 and `.data` at the page-aligned `data_base`.
4. Apply absolute data relocations against the final mapping address.
5. Write a one-instruction trampoline at the end of `.text`:
   `b <entry>`. It is reached with `blr`, so `x30` becomes the return address
   and no general-purpose register is clobbered.
6. Change the code region to `PROT_READ | PROT_EXEC`; keep the data region
   `PROT_READ | PROT_WRITE`. Android's W^X policy is respected.
7. Flush the instruction cache (`dc cvau`, `dsb ish`, `ic ivau`, `dsb ish`,
   `isb`).
8. Allocate a private 64 KiB guest stack with a `PROT_NONE` guard page.
9. Call the naked assembly trampoline in `exec.rs`.

The guest register file is a `[u64; 33]` (x0–x30, SP, NZCV). The trampoline
saves the host callee-saved registers, installs the guest SP, loads the guest
registers, and branches through the trampoline. On return it saves the guest
registers, SP and NZCV back into the array and restores the host registers in
reverse order.

Guest `x30` is special: the guest returns through the host link register, so
after the run `x30` is the host return address, not a guest value. This is
reported explicitly in the UI and documented in
[LIMITATIONS.md](LIMITATIONS.md).

Raw hex runs use the same path with an empty data section. The entry offset
must be 4-byte aligned and inside the payload.

## Timeouts and recovery

There are two deadlines:

- **Service watchdog** (`AssemblerService`): a `Handler` on the worker's main
  looper posts a kill at `timeoutMs` (clamped 200 ms–60 s). It writes the
  timeout marker and calls `Process.killProcess(Process.myPid())`.
- **Client deadline** (`WorkerClient`): `timeoutMs + 1500 ms`. If no reply
  arrives, it checks the marker. If the marker exists, the UI sees
  `worker was terminated after the timeout`; otherwise it reports that the
  worker did not answer.

Source runs containing `svc` are rejected before the worker starts. The app
does not emulate Linux syscalls or expose guest stdout, so source programs
must return through `ret` and expose results through the captured registers.

A guest crash kills only the worker. `onServiceDisconnected` /
`onBindingDied` clear the binding, fail the pending request, unbind, and stop
the service so it is not restarted in a loop. The next run binds a fresh
worker.

## UI state

`StudioViewModel` owns the editor source, selection, diagnostics, current
project, execution result, theme and token colors as `StateFlow`s. Screens are
plain Compose functions selected by a `Screen` enum; there is no navigation
library. The editor is a `BasicTextField` with a `VisualTransformation` for
syntax highlighting and error underlines, plus a minimap, key row and
line-number gutter.

Projects are plain `.s` files in the app's private `files/projects/` directory.
The bundled examples are copied there on first launch. External text files are
selected through Android's `OpenDocument` picker and copied into this directory
before they are opened; no external URI permission is retained. No database
and no network are involved.
