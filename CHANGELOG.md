# Changelog

## 0.1.0

- Initial release of A64 Studio.
- Native AArch64 assembler with 270 encodings verified against LLVM.
- Real execution on ARM64 Android in an isolated `:worker` process with a
  watchdog and crash recovery.
- Editor with syntax highlighting, diagnostics, folding, find/replace,
  undo/redo, autocomplete, signature hints, minimap and custom key row.
- Registers, machine code, memory, CPU Lab, instruction explorer, hex patcher,
  benchmarks, projects and appearance screens.
- Bundled examples: `hello`, `registers`, `fib`, `neon`, `memory`.
- Offline by design: no `INTERNET` permission.
- MPL-2.0, F-Droid metadata, CI and verification documentation.
