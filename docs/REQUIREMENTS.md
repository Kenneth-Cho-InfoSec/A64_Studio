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
| R-01 | Kotlin + Compose Android UI with a large editor at launch | Implemented | `app/src/main/java/dev/a64studio/ui/` |
| R-02 | Offline Rust AArch64 assembler with diagnostics and source mappings | Implemented / Verified | `core/src/asm/`; 270-form LLVM check passes locally |
| R-03 | Real native execution on ARM64, never emulation | Implemented / device-reported | `core/src/exec.rs`; device evidence is recorded in `docs/VERIFICATION.md` |
| R-04 | Isolated worker process, watchdog, crash/timeout reporting | Implemented / device-reported | `AssemblerService`, `WorkerClient`; final instrumentation rerun pending |
| R-05 | Syntax highlighting, diagnostics, editor navigation, folding and key row | Implemented | `core/src/lexer.rs`, `EditorScreen.kt`, `Highlight.kt` |
| R-06 | Register, machine-code, memory, CPU and project views | Implemented | `InspectorScreens.kt`, `CpuLabScreen.kt`, `ProjectsScreen.kt` |
| R-07 | Instruction explorer, hex patcher, benchmarks and bundled examples | Implemented | corresponding Compose screens and `core/src/bench.rs` |
| R-08 | No network permission, root, Termux, or external runtime dependency | Verified / Limited | manifest, build metadata, and APK evidence; device rerun pending |
| R-09 | Project save/reopen and private `.s` storage | Implemented / device-reported | `ProjectStore.kt`; instrumentation source exists |
| R-10 | ARM64-only APK packaging and F-Droid-compatible open-source metadata | Implemented | Gradle ABI filter, GPL-3.0, `metadata/`, CI |

## Non-functional requirements

| ID | Requirement | Status |
|---|---|---|
| N-01 | Keep native logic in Rust and Android UI/lifecycle in Kotlin | Verified by tree inspection |
| N-02 | Preserve W^X separation and document execution isolation limits | Implemented; documented |
| N-03 | Make unsupported instructions and platforms explicit | Implemented; tested on x86 host |
| N-04 | Keep builds reproducible with Cargo/Gradle lockfiles and CI | Implemented; clean local Gradle rerun pending in this sandbox |
| N-05 | Keep agent context portable across development tooling/development tooling | In progress; this checkpoint system completes the first version |

## Explicit limitations, not missing requirements

Complete GNU assembler coverage, C preprocessing, macros, external symbols,
ELF/object output, debugger stepping, live memory editing, SIMD register
capture, and a security sandbox were explicitly excluded or documented as
unsupported by the original implementation. Do not turn these into backlog
items without a new product decision.
