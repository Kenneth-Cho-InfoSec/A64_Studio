# Limitations

A64 Studio tries to be explicit about what it cannot do. These are real
constraints, not placeholders.

## Platform

- **ARM64 Android only.** The APK contains `arm64-v8a` only. On an x86_64
  emulator or an x86_64 Linux host, assembly, disassembly and syntax
  highlighting work, but execution returns:
  `native execution is unavailable on this host: A64 Studio is running on a
  non-ARM64 CPU`.
- **Android 8.0 (API 26) or newer.**
- **ARM64-on-x86_64 emulator testing uses an unsupported workaround.** The normal
  Android emulator frontend rejects ARM64 system images on x86_64. A direct
  headless TCG run of the API 26 image was used only for Android integration
  testing. API 28/30/36 images crashed before a usable boot in this
  environment. Treat emulator results as Android-lifecycle evidence, not as
  real-CPU evidence.
- **No root, no Termux, no Linux userland.** Guest code runs in the app's
  ordinary Android sandbox.

## Execution model

- **No emulation.** There is no QEMU, Unicorn, box64 or software CPU in the
  app. The QEMU script in `scripts/` is only a desktop/CI fallback for the Rust
  tests; it is not part of the APK and is not a substitute for a device.
- **Guest `x30` is the host return address.** The guest is entered with
  `blr`, so `x30` holds the trampoline return address. After the run the
  Registers screen reports that host address for `x30`, and the result note
  says so. A guest that needs its own `x30` value must save it somewhere
  else before returning.
- **The guest is a leaf function.** A64 Studio provides a private 64 KiB stack
  with a guard page, but no libc, no dynamic linker, no thread setup and no
  syscall ABI. Guest code should compute a result and `ret`. Calling into
  Android libraries or starting threads is not supported.
- **Self-modifying code is not supported.** Code pages are `RX` and data pages
  are `RW`; A64 Studio does not switch permissions at runtime or bypass
  Android's W^X policy.
- **The timeout is enforced by the Android worker, not by the Rust core.**
  The Rust execution function has no preemption. In the app, the `:worker`
  service's watchdog kills the process at the deadline (clamped to
  200 ms–60 s). A desktop `cargo test` or a direct JNI call has no watchdog;
  an infinite guest loop will hang that process. The QEMU test script only
  runs the finite test suite.
- **No live memory inspection.** The Memory screen is a snapshot of the image
  after the guest returns (first 64 KiB). It is not a live debugger and cannot
  modify a running guest.
- **No SIMD/FP register inspector.** The execution result captures x0–x30, SP
  and NZCV. NEON/FP instructions assemble and execute, but V0–V31, FPCR and
  FPSR are not shown after the run.
- **No stepping, breakpoints or watchpoints.** There is no debugger.
- **No relocatable objects or ELF output.** The assembler produces a flat
  in-memory image, not an object file or a linked executable.

## Assembler coverage

- The encoder is a focused hand-written AArch64 assembler with **270 verified
  encodings**, not a complete GNU assembler. Unsupported mnemonics, operand
  forms, immediate ranges, shifts and extensions produce diagnostics. They are
  never silently encoded as something else.
- **No C preprocessor.** `.S` files with `#include`, `#define` or `#if` are
  rejected with a message telling you to save as `.s` and remove them.
- **No macros or repetition.** `.macro`, `.endm`, `.rept`, `.endr`, `.if`,
  `.ifdef`, `.include` and `.org` are explicitly unsupported.
- **No external symbols.** Only labels defined in the current file can be
  referenced. There is no linker and no library.
- **Literal pools are limited** to the forms the assembler emits for `ldr`
  pseudo-instructions; arbitrary hand-written pools are not guaranteed.
- **`.bss`/`.rodata` are placed in the writable data image.** They are not
  given separate permissions.
- **Encoding verification is a snapshot.** `scripts/verify-encodings.py`
  checks the current table against the NDK's LLVM. It does not prove that
  every AArch64 instruction or alias is covered.

## UI and storage

- Projects are plain `.s` files in the app's private storage. There is no
  cloud sync, no Git integration and no import/export picker beyond the app's
  own project list.
- The editor is a `BasicTextField` with a visual transformation. Very large
  files (tens of thousands of lines) will be slower than a native editor.
- Syntax highlighting, folding, autocomplete and signature hints are
  token/line based. They are not a compiler front end.
- The minimap is a lightweight line-density map, not a rendered code
  thumbnail.
- There is no undo history persistence across app restarts.

## Security and isolation

- The `:worker` process is **process isolation, not a security sandbox**. It
  runs with the same Android UID and app sandbox as the editor. Guest code can
  attempt syscalls and can read files the app can read. The watchdog protects
  against hangs and crashes; it does not protect against malicious code.
- The app requests no `INTERNET` permission and has no networking code, but
  this is not a substitute for not running untrusted assembly.
- A guest crash kills the worker. The editor survives, but unsaved editor
  changes are not written automatically unless you saved them.
- The release APK in this repository was signed with a local development key.
  It is not an official production key. F-Droid signs with its own key.

## Verification gaps

- The real-device checks were performed on a Pixel 8 (Android 17,
  `arm64-v8a`). Other ARM64 devices, page sizes and Android versions may
  expose different HWCAP features or W^X behavior.
- The Android instrumentation suite passes on an ARM64 API 26 emulator (TCG):
  **7/7 tests**, including the x1–x29 zero assertion after
  `mov x0, #42; ret`. This validates the Android/JNI/Binder/`:worker`/Compose
  path, but not real-hardware CPU timing. A physical-device re-run after the
  trampoline fix is still pending.
- The API 26 emulator was the only ARM64 image that completed boot in this
  environment. API 28 failed in `system_server` (Codec2), and API 30/36 host
  QEMU processes crashed in `goldfish_address_space_memory_state_save`.
- QEMU user-mode validates the AArch64 trampoline and Rust tests. It does not
  validate Android `:worker` lifecycle, Binder behavior or Compose UI; the
  API 26 emulator covers those.
- Gradle's `connectedDebugAndroidTest` task was not used because its UTP
  dependency was unavailable offline. The same test APK was installed and run
  with `adb shell am instrument`; this does not change the test code or result.
