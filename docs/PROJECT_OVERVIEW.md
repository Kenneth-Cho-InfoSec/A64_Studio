# A64 Studio project overview

## Product

A64 Studio is an offline Android IDE for writing, assembling, inspecting, and
executing small AArch64 programs. On a compatible ARM64 Android device, guest
machine code runs directly on the device CPU inside a separate `:worker`
process with a watchdog. It is not an emulator, a general compiler, a Linux
shell, or a security sandbox.

## Intended users

The primary users are developers, students, and low-level systems enthusiasts
who want to experiment with AArch64 instructions on an Android ARM64 device
without root, Termux, a network connection, or an external toolchain.

## Core experience

1. Open an example or edit a `.s` source file in the Compose editor.
2. Receive offline syntax highlighting, diagnostics, and assembly results.
3. Inspect source-to-byte mappings, disassembly, symbols, relocations, and a
   bounded post-run memory image.
4. Run the image in the isolated worker and inspect x0–x30, SP, and NZCV.
5. Explore CPU capabilities, supported instruction metadata, benchmarks, and
   raw-hex patching.

## Product boundary

The assembler is a documented, hand-written subset with 270 encoding forms
cross-checked against the NDK LLVM assembler. Unsupported syntax is diagnosed.
The app does not promise complete GNU assembler, ELF/linker output, debugger
stepping, arbitrary process access, or SIMD register capture. See
`docs/LIMITATIONS.md` for the full boundary.

## Current release shape

The repository contains the 0.1.0 implementation: Kotlin/Jetpack Compose UI,
Rust `cdylib`, JNI JSON bridge, worker-process execution, bundled examples,
tests, CI, and F-Droid metadata. The implementation status and evidence are
tracked separately in `docs/PROJECT_STATUS.md`; this file describes intent,
not proof of every feature.
