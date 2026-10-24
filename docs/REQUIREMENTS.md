# A64 Studio requirements

Status labels mean:

- **Verified** — supported by a reproducible local command or recorded device
  evidence.
- **Implemented** — present in source but not fully re-verified in the current
  environment.
- **Pending** — required or useful work remains.
- **Limited** — intentionally outside the product boundary.

The original development tooling user brief is the source for product requirements. Items
marked “inferred” are conclusions from the source tree, not new promises.

## Functional requirements

| ID | Requirement | Status | Evidence / note |
|---|---|---|---|
| R-01 | Kotlin + Compose Android UI with a large editor at launch | Verified | `app/src/main/java/dev/a64studio/ui/`; UI launched on the API 26 ARM64 emulator and Pixel 8 |
| R-02 | Offline Rust AArch64 assembler with diagnostics and source mappings | Verified | `core/src/asm/`; 270/270 LLVM encoding check and Rust tests |
| R-03 | Real native execution on ARM64, never emulation | Verified | `core/src/exec.rs`; Pixel 8 and API 26 ARM64 emulator evidence in `docs/VERIFICATION.md` |
| R-04 | Isolated worker process, watchdog, crash/timeout reporting | Verified | `AssemblerService`, `WorkerClient`; 7/7 instrumentation including `b .` timeout |
| R-05 | Syntax highlighting, diagnostics, editor navigation, folding and key row | Implemented / verified | `core/src/lexer.rs`, `EditorScreen.kt`, `Highlight.kt`; 5/5 JVM tests and manual UI |
| R-06 | Register, machine-code, memory, CPU and project views | Implemented / partially verified | `InspectorScreens.kt`, `CpuLabScreen.kt`, `ProjectsScreen.kt`; registers and project storage exercised on API 26, other inspector screens source-reviewed |
| R-07 | Instruction explorer, hex patcher, benchmarks and bundled examples | Implemented / partially verified | corresponding Compose screens and `core/src/bench.rs`; examples packaged and Fibonacci run, other screens source-reviewed |
| R-08 | No network permission, root, Termux, or external runtime dependency | Verified | manifest, build metadata, and APK inspection; no `INTERNET` permission |
| R-09 | Project save/reopen and private `.s` storage | Verified | `ProjectStore.kt`; `saveAndReopenProject` instrumentation test passes |
| R-10 | ARM64-only APK packaging and F-Droid-compatible open-source metadata | Verified | Gradle ABI filter, GPL-3.0, `metadata/`, CI; APK inspection |

## Non-functional requirements

| ID | Requirement | Status |
|---|---|---|
| N-01 | Keep native logic in Rust and Android UI/lifecycle in Kotlin | Verified by tree inspection and build |
| N-02 | Preserve W^X separation and document execution isolation limits | Implemented; documented in `docs/LIMITATIONS.md` |
| N-03 | Make unsupported instructions and platforms explicit | Verified on x86_64 host and ARM64 emulator |
| N-04 | Keep builds reproducible with Cargo/Gradle lockfiles and CI | Verified in current workspace: clean offline Gradle build and committed lockfiles; release signing uses a local untracked key |
| N-05 | Keep agent context portable across development tooling/development tooling | Implemented; `scripts/project-state.py` and `docs/PROJECT_STATUS.md` verified locally |

## Explicit limitations, not missing requirements

Complete GNU assembler coverage, C preprocessing, macros, external symbols,
ELF/object output, debugger stepping, live memory editing, SIMD register
capture, and a security sandbox were explicitly excluded or documented as
unsupported by the original implementation. Do not turn these into backlog
items without a new product decision.
