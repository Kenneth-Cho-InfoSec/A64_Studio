# A64 Studio

<p align="center">
  <img src="app/src/main/res/drawable-nodpi/ic_launcher_logo.png" alt="A64 Studio logo" width="220">
</p>

<p align="center">
  A native AArch64 assembly studio for Android
</p>

A64 Studio is an offline Android IDE for writing, assembling, inspecting, and
executing AArch64 assembly on a compatible device. The Rust core assembles the
source and the Android application runs the resulting instructions directly on
the device CPU. The app does not emulate ARM, bundle a Linux userland, or send
source code to a server.

The project is designed for learning, experimentation, instruction encoding
work, register inspection, and small native execution experiments on real
ARM64 Android hardware.

## Project status

A64 Studio is an early 0.1.0 release. The editor, assembler, disassembler,
native execution path, project storage, appearance controls, diagnostic tools,
and device validation workflow are implemented. The assembler intentionally
covers a focused set of AArch64 instructions rather than claiming full GNU
assembler compatibility.

The current source tree has been validated with:

- 49 Rust unit tests and one bundled-example test.
- 5 Kotlin/JVM tests.
- 9 Android instrumentation tests on a physical Pixel 8.
- Native execution on an ARM64 Android device.
- QEMU user-mode validation of the Rust AArch64 execution path.
- An LLVM encoding comparison covering 270 instruction forms.

See [docs/VERIFICATION.md](docs/VERIFICATION.md) for command output, device
details, APK hashes, emulator limitations, and the distinction between host,
QEMU, emulator, and physical-device evidence.

## Features

### Assembly editor

- Monospace AArch64 editor built with Jetpack Compose.
- Line numbers aligned to the measured text layout.
- Syntax highlighting driven by the Rust lexer.
- Error and warning underlines with line and column diagnostics.
- Label highlighting and active-label feedback.
- Autocomplete suggestions for instructions, registers, conditions, and labels.
- Signature hints for recognized instructions.
- Find, replace-one, replace-all, and cursor navigation.
- Undo and redo history.
- Source folding for labeled blocks.
- A minimap aligned to the editor's measured line geometry.
- A compact assembly key row for common symbols and registers.
- Import of external `.s` files through the Android document picker.
- Large pasted files are handled without crashing the gutter or minimap.

### Assemble, run, and inspect

- Assemble source in the editor process for fast diagnostics.
- Run native code in a separate Android `:worker` process.
- Stop a running guest program.
- Capture general-purpose registers `x0` through `x30`, SP, and NZCV.
- Capture elapsed execution time and the guest exit status.
- Inspect encoded machine code and source mappings.
- Inspect the loaded code and data image.
- View a bounded hexadecimal memory dump after execution.
- Explore supported instruction encodings and examples.
- Run raw little-endian AArch64 payloads with an entry offset.
- Run bundled integer, memory, NEON, and floating-point benchmarks.

### Projects and examples

Projects are stored as plain `.s` files in the app's private storage. The
project menu supports creating, opening, saving, importing, renaming, and
deleting projects. Long-press the centered project filename in the overflow
menu to rename the active project.

Bundled examples include:

- `hello.s`
- `registers.s`
- `fib.s`
- `neon.s`
- `memory.s`

### Appearance and accessibility

- System, light, and dark appearance modes.
- OLED black default palette for the Midnight theme.
- Custom accent and token colors.
- Persisted editor font size.
- Material 3 color roles, surfaces, shapes, and component states.
- Accessible labels for editor, minimap, actions, diagnostics, and navigation.
- Responsive bottom navigation and navigation rail behavior.
- Touch targets sized for the Android interface where the compact editor
  layout allows it.

### About and licensing

The app includes an About screen with developer information, runtime license
information, project links, and the application license. A64 Studio is
released under the Mozilla Public License 2.0.

## Execution model

The execution path is intentionally simple and explicit:

```text
Compose editor
    |
    | JNI and JSON result transport
    v
Rust core in liba64core.so
    |
    | Android Messenger IPC
    v
Private :worker process
    |
    +-- native AArch64 execution
    +-- RX code mapping
    +-- RW data mapping
    +-- watchdog process termination
```

The editor process does not execute guest instructions directly. The worker
process has a watchdog that terminates a guest which does not return before its
deadline. This protects the editor process from infinite loops and many guest
crashes. It is process isolation, not a complete security sandbox.

### Return contract

A source program must eventually return through `ret` so the execution
trampoline can capture its register state. Results are exposed through the
Registers, Machine Code, and Memory screens.

A64 Studio does not provide a Linux syscall environment or a guest stdout
stream. Source programs that contain `svc` are rejected before entering the
worker and report an unsupported-instruction result. This means Linux-style
programs that use `svc` for `write` or `exit` must be adapted to return values
through registers instead.

For example:

```asm
.text
.global asm_main

asm_main:
    mov x0, #42
    ret
```

After running, open Registers to inspect `x0`.

## Platform requirements

- Android 8.0 or newer, API 26 or newer.
- A physical ARM64 device with the `arm64-v8a` ABI for native execution.
- Android SDK platform 36 for the current build configuration.
- Android NDK `28.2.13676358`.
- JDK 17 or newer. JDK 21 is used by the current local release workflow.
- Rust 1.82 or newer for the core toolchain.
- The Android app does not require root, Termux, network access, or a remote
  execution service.

The APK contains only the `arm64-v8a` native library. On an x86 or x86_64
device, the UI and offline tools can still open, but native execution reports
that ARM64 execution is unavailable. A64 Studio never substitutes emulation
for native execution in the app.

## Build from source

Clone the repository, open a terminal at its root, and make sure the Android
SDK, NDK, JDK, Rust toolchain, and Android Rust target are available.

```bash
rustup target add aarch64-linux-android
./gradlew :app:assembleDebug
```

The Gradle build cross-compiles the Rust core and copies the resulting shared
library into the Android application. The build configures the Android NDK
linker and uses a 16 KiB maximum page size linker argument for newer Android
devices.

The debug APK is written to:

```text
app/build/outputs/apk/debug/app-debug.apk
```

The debug application ID is `dev.a64studio.debug`.

Build a release APK with:

```bash
./gradlew :app:assembleRelease
```

The release APK is written to:

```text
app/build/outputs/apk/release/app-release.apk
```

### Local release signing

The repository does not contain a signing key. To create a local signed build,
provide a keystore and the expected environment variables:

```bash
keytool -genkeypair \
  -keystore "$HOME/.a64studio-release.jks" \
  -alias a64studio \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000

export A64_RELEASE_STORE_FILE="$HOME/.a64studio-release.jks"
export A64_RELEASE_STORE_PASSWORD='your-store-password'
export A64_RELEASE_KEY_ALIAS=a64studio
export A64_RELEASE_KEY_PASSWORD='your-key-password'

./gradlew :app:assembleRelease
```

Do not publish a locally generated signing key as the official application
release key. F-Droid and other distribution services should sign their own
artifacts.

## Install and run on a Pixel 8

With USB debugging enabled and the phone connected:

```bash
adb devices
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell monkey -p dev.a64studio.debug 1
```

To run the instrumentation APK directly when the offline Gradle connected-test
plugin is unavailable:

```bash
./gradlew :app:assembleDebugAndroidTest
adb install -r app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell am instrument -w -r \
  dev.a64studio.debug.test/androidx.test.runner.AndroidJUnitRunner
```

The current test suite includes native execution, raw payload execution,
diagnostics, disassembly, project storage, external-import behavior, worker
watchdog behavior, unsupported syscall handling, and the no-network permission
check.

## Tests and validation

Run the Rust core tests:

```bash
cd core
cargo test
cd ..
```

Run Kotlin/JVM tests:

```bash
./gradlew :app:testDebugUnitTest --offline
```

Run the Android instrumentation suite on a connected ARM64 device:

```bash
./gradlew :app:connectedDebugAndroidTest
```

Run the LLVM encoding comparison when an Android NDK toolchain is available:

```bash
python3 scripts/verify-encodings.py
```

Run the QEMU user-mode Rust validation when the target and QEMU are installed:

```bash
rustup target add aarch64-unknown-linux-musl
scripts/qemu-aarch64-test.sh
```

QEMU validation is useful for the Rust execution and trampoline code. It is
not evidence that the Android app can execute natively on a non-ARM64 phone.

## Repository layout

```text
app/
  src/main/java/dev/a64studio/
    core/       JNI bridge and native result models
    storage/    private project storage and imports
    ui/         Compose screens, editor, theme, and navigation
    worker/     isolated execution service and IPC client
  src/main/assets/examples/
  src/androidTest/
  src/test/
core/
  src/          Rust assembler, lexer, disassembler, execution, and benches
docs/
  PROJECT_OVERVIEW.md
  REQUIREMENTS.md
  ARCHITECTURE.md
  PROJECT_STATUS.md
  TASKS.md
  PROJECT_STATUS.md
  DECISIONS.md
  VERIFICATION.md
scripts/
  verify-encodings.py
  qemu-aarch64-test.sh
  project-state.py
```

## Documentation

- [Project overview](docs/PROJECT_OVERVIEW.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Project status](docs/PROJECT_STATUS.md)
- [Development tasks](docs/TASKS.md)
- [Agent checkpoint](docs/PROJECT_STATUS.md)
- [Architecture decisions](docs/DECISIONS.md)
- [Verification record](docs/VERIFICATION.md)
- [Known limitations](docs/LIMITATIONS.md)
- [Dependency licenses](docs/DEPENDENCIES.md)
- [Material 3 UI plan](UI_UX_MATERIAL3_IMPROVEMENT_PLAN.md)

The repository also contains `CONTRIBUTING.md`, which defines the portable workflow
for development tooling, development tooling, external model, and other coding agents working on the same
checkout.

## Architecture and technical decisions

The project uses a two-pass hand-written Rust assembler. It supports labels,
sections, selected data directives, relocations, diagnostics, and a focused
set of AArch64 encodings. The implementation is cross-checked against the
NDK LLVM assembler by `scripts/verify-encodings.py`.

The editor is written in Kotlin with Jetpack Compose and Material 3. The app
keeps the existing enum-based navigation model, uses a bottom navigation bar
on compact windows, and switches to a navigation rail at wider widths. The
Rust core stays separate from Android UI and lifecycle code.

Native execution uses a private memory image, executable code pages, writable
data pages, a private guest stack, an AArch64 trampoline, and a worker process
watchdog. These boundaries are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
and [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

## Known limitations

A64 Studio is not a complete GNU assembler, linker, debugger, or operating
system environment. The following are outside the current implementation:

- Full GNU assembler syntax and macro preprocessing.
- C preprocessing, include files, external object files, and ELF output.
- Source-level stepping and breakpoints.
- Linux syscall emulation and guest stdout.
- A security sandbox for untrusted native code.
- Live memory editing during execution.
- Complete SIMD register capture.
- Native execution on non-ARM64 devices.

Unsupported instructions, directives, operands, and immediate ranges should
produce diagnostics rather than silently generating incorrect machine code.

## Contributing

Before changing code:

1. Read `CONTRIBUTING.md` and `docs/PROJECT_STATUS.md`.
2. Check the Git status and recent commits.
3. Read the active task, requirements, and relevant source files.
4. Preserve existing uncommitted work from other agents.
5. Make focused changes and run the smallest relevant tests.

Before handing work to another agent:

1. Update `docs/TASKS.md` and `docs/PROJECT_STATUS.md`.
2. Record changed files and validation results.
3. Document blockers and unverified claims.
4. Leave precise next steps for development tooling, development tooling, or another contributor.

Git is the canonical record of code changes. The repository documents are the
portable context layer that allows development to continue without access to a
previous AI conversation.

## License

A64 Studio is released under the [Mozilla Public License 2.0](LICENSE).
Third-party dependency notices are available in
[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
