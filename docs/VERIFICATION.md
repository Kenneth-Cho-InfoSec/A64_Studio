# Verification

This document records what was actually tested, on which hardware or emulator,
and what was only tested in a fallback environment. It deliberately separates
real-device evidence, Android ARM64 emulator evidence, and host/QEMU evidence.

## Real device

- Device: Google Pixel 8
- Android: 17
- ABI: `arm64-v8a`
- Page size: 4096 bytes (`getconf PAGE_SIZE`)
- App: debug build (`dev.a64studio.debug`)
- Connection: USB debugging, no root, no emulator

The original manual checks below were performed before the final trampoline
fix. The final APK was subsequently installed on this same Pixel and the full
post-fix instrumentation suite passed, including the x1–x29 register check.

Commands used for the basic device checks:

```bash
adb devices
adb shell getprop ro.product.cpu.abi
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb install -r app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb shell am instrument -w -r \
  dev.a64studio.debug.test/androidx.test.runner.AndroidJUnitRunner
adb shell am start -n dev.a64studio.debug/dev.a64studio.MainActivity
```

The post-fix device run used serial `37261FDJH0033X` and completed with:

```
Time: 2.588
OK (7 tests)
```

### Current Material 3 UI milestone

The approved UI implementation was rebuilt from commit `d5cf3cb` and checked
on the same physical Pixel 8 without an emulator:

```text
release build: BUILD SUCCESSFUL
debug + androidTest build: BUILD SUCCESSFUL
Rust: 48 unit tests + 1 bundled-examples test passed
JVM: 5/5 tests passed
instrumentation: Time: 2.553, OK (7 tests)
```

The release APK installed successfully. Manual device checks verified:

- the bottom navigation now displays Material icons with labels;
- the editor exposes `Assembly editor` and `Code minimap preview` semantics;
- Projects opens a secondary screen with a labeled `Back` action, and Android
  Back returns to Editor;
- the Integer ALU benchmark completed and displayed its result;
- the editor run state remained functional and displayed the assembled source.

Current signed release APK SHA-256:

```text
8051f5d356c9b4ec877aaaa85c5afcaa0bd3b0885fcb3633a1ef6236e10a6a16
```

This validates the implemented UI milestone on a compact Pixel window. The
NavigationRail branch is source-verified but remains untested on a physical
600 dp or wider window in this environment.

The suite included `movX0_42_returns42`, which asserted `x0 = 42` and every
register x1–x29 was zero, including x10. The debug APK installed successfully
from `dist/a64studio-0.1.0-debug.apk`.

### 1. Raw hex execution: `mov x0, #42; ret`

The Hex Patcher was given the little-endian payload:

```
40 05 80 d2 c0 03 5f d6
```

Entry offset `0`. The run completed and the Registers screen showed:

```
x0 = 0x000000000000002a
```

This is the required `mov x0,#42; ret` result of 42, executed by the Pixel's
real CPU through the worker process.

### 2. Source execution: Fibonacci

The bundled `fib.s` example was assembled and run. The Registers screen
showed:

```
x0 = 0x0000000000000037   (55)
x1 = 0x0000000000000059   (89)
```

The editor status line reported a completed run. This exercises the
assembler, relocation/layout path, `:worker` IPC, native execution and result
parsing together.

### 3. Timeout and crash recovery

The Hex Patcher was given an infinite branch:

```
00 00 00 14        // b .
```

The worker watchdog fired. The editor process stayed alive, the pending run
was failed, and the status line reported:

```
worker was terminated after the timeout
```

This is the required worker-hang test. It demonstrates that a non-returning
guest is killed in the `:worker` process rather than blocking the UI.

### 4. Diagnostics and error underline

The editor was given:

```asm
.text
.global asm_main
asm_main:
    mov x0, #notanumber
    ret
```

Assembly failed with one error. The editor drew a red underline under
`#notanumber` and displayed:

```
line 4:12  literal expression: undefined symbol 'notanumber'
```

The diagnostic line/column points at the invalid operand, not at the start of
the file.

### 5. Offline / permissions

The packaged APK was inspected with the Android build tools:

```bash
aapt dump permissions app/build/outputs/apk/debug/app-debug.apk
aapt dump badging app/build/outputs/apk/release/app-release.apk
```

The only permission is the Android-generated
`DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`. There is **no**
`android.permission.INTERNET`. The release badging reports:

```
package: name='dev.a64studio' versionCode='1' versionName='0.1.0'
sdkVersion:'26'
targetSdkVersion:'36'
native-code: 'arm64-v8a'
```

### 6. Screenshots

The Pixel screenshots in `docs/screenshots/` are:

- `editor-fib.png` — Fibonacci source and completed run.
- `registers-fib.png` — register inspector after Fibonacci.
- `diagnostics.png` — invalid immediate, underline and diagnostic.
- `hex-patcher.png` — raw-hex disassembly/execution screen.
- `worker-timeout.png` — status after the infinite branch was killed.

The API 26 emulator screenshot is
`docs/screenshots/registers-fib-api26.png`; it is described in the emulator
section below.

### 7. Register-leak finding

The `registers-fib.png` capture also exposed a bug: guest `x10` contained a
host pointer (`0xd381a83dbf90`) instead of zero. The trampoline had used
`x10` as the register-file base and loaded guest `x10` too early. The
trampoline now loads `x10` last and uses `x30` for the trampoline pointer.

The final Android test now asserts that **x1–x29 are zero** after
`mov x0, #42; ret`, not just `x10`. It passes on both the API 26 ARM64
emulator and the physical Pixel 8. The fix is included in the rebuilt debug
and release APKs. The Pixel result is instrumentation evidence; it does not
claim that the manual pre-fix screenshot has been corrected.

### 8. Post-fix release smoke test

The final release APK was installed on the Pixel 8 and launched successfully
with `am start -W`. The bundled `fib` source was run from the editor; the UI
reported `Run completed in 0.004 ms`, and the Registers screen showed
`x0 = 0x37` (55), `x1 = 0x59` (89), and `x10 = 0x0`.

The same release session opened the Machine, Memory, CPU Lab, Instruction
Explorer, Hex Patcher, and Benchmarks screens. The Hex Patcher executed
`400580d2c0035fd6` and reported `Raw run completed` with `x0 =
0x000000000000002a`. Instruction Explorer displayed 132 implemented
instructions. CPU Lab reported native ARM64 execution available, `aarch64`,
9 logical cores, and a 4096-byte page size. The Integer ALU guest benchmark
completed with `5,000,000` iterations in `6.63 ms` (753,906,631.1
iterations/s). No app fatal-exception or native-crash indicator appeared in
the inspected logcat output after these interactions.

## Android ARM64 emulator (TCG)

The Android emulator frontend does not support ARM64 system images on an
x86_64 host. A direct headless QEMU invocation was used as a workaround. This
is an **emulated ARM64 Android guest**, not the Pixel's real CPU. It validates
the Android/JNI/Binder/`:worker`/Compose integration path that the host tests
cannot cover; it does not replace real-hardware CPU or performance evidence.

- Host: Arch Linux x86_64
- Emulator: Android Emulator 37.2.12.0 (build 16428233)
- Binary: `emulator/qemu/linux-x86_64/qemu-system-aarch64-headless`
- AVD: `a64test26`
- System image: `system-images;android-26;default;arm64-v8a`
- Android: 8.0.0 (API 26)
- ABI: `arm64-v8a`
- Resources: 4 cores, 2048 MB RAM, no snapshot, software GPU (`-gpu off`),
  headless
- ADB serial used: `emulator-5556`

Boot command:

```bash
$ANDROID_HOME/emulator/qemu/linux-x86_64/qemu-system-aarch64-headless \
  -avd a64test26 \
  -no-window -no-audio -no-boot-anim \
  -gpu off -no-snapshot -cores 4 -memory 2048 -no-metrics \
  -show-kernel -qemu -machine virt
```

The guest reached `sys.boot_completed=1` and stayed up. It reported
`ro.build.version.release=8.0.0` and `ro.product.cpu.abi=arm64-v8a`.

### Instrumentation suite

The test APK was installed and run directly with `adb`. Gradle's
`connectedDebugAndroidTest` task was not used because its UTP dependency could
not be resolved in the offline environment; the same test APK and runner were
used.

```bash
adb -s emulator-5556 install -r app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5556 install -r app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb -s emulator-5556 shell am instrument -w -r \
  dev.a64studio.debug.test/androidx.test.runner.AndroidJUnitRunner
```

Result:

```
Time: 1.989
OK (7 tests)
```

The seven tests cover:

| Test | Evidence |
|---|---|
| `movX0_42_returns42` | Source `mov x0, #42; ret` returns `x0 = 42`; x1–x29 are all zero, including the former `x10` leak. |
| `rawHexMovX0_42_returns42` | Raw payload `400580d2c0035fd6` returns `x0 = 42`. |
| `saveAndReopenProject` | Project source round-trips through private `.s` storage. |
| `assemblyDiagnosticsHaveLineAndColumn` | Invalid input produces a diagnostic with line and column. |
| `hangingGuestIsTerminatedByWorkerWatchdog` | `b .` is killed by the worker watchdog; the editor process survives. |
| `appRequestsNoNetworkPermission` | The installed app does not request `INTERNET`. |
| `disassemblerDecodesMovAndRet` | The disassembler decodes the expected `mov`/`ret` instructions. |

Test APK: `app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk`
(383,483 bytes, SHA-256
`fb87f8b086180cb5e6f6f2fd95618e787dfa24fea00b98d46d7b9a0d677db5e8`).

### Manual UI and release APK

The debug build was launched and the bundled `fib.s` example was assembled and
run. The Registers screen showed:

```
x0 = 0x37   (55)
x1 = 0x59   (89)
```

The release APK was then installed and launched on the same emulator:

```bash
adb -s emulator-5556 install -r dist/a64studio-0.1.0-release.apk
adb -s emulator-5556 shell am start -n dev.a64studio/dev.a64studio.MainActivity
```

`fib.s` ran from the release build with `Run completed in 0.180 ms`, and the
Registers screen showed the same `x0=0x37`, `x1=0x59` values. The screenshot
`docs/screenshots/registers-fib-api26.png` is from that release run.

### Emulator boundary

- This is TCG emulation of an ARM64 Android guest. Timings such as 0.180 ms
  are guest-emulation timings, not Pixel hardware timings.
- API 28 booted to adb, but `system_server` boot-looped after a SIGSEGV in
  `libstagefright_codec2_vndk.so` (`_GLOBAL__sub_I_C2Config.cpp`); it never
  reached a usable `sys.boot_completed=1` state.
- API 30 and API 36 host QEMU processes crashed with SIGSEGV in
  `goldfish_address_space_memory_state_save`.
- API 26 was the only image that completed boot and remained usable in this
  environment. No claim is made for other Android versions on this emulator.

## Host and QEMU

These checks do not replace a device test. They validate the Rust core and
the AArch64 code path when no device is attached.

### Rust tests on x86_64

```bash
cd core
cargo test
```

Result:

```
running 48 tests
test result: ok. 48 passed; 0 failed
running 0 tests
test result: ok. 0 passed
test result: ok. 1 passed        # tests/examples.rs
```

The host is x86_64, so the execution test asserts the explicit
"non-ARM64" unsupported result instead of running guest code.

### QEMU user-mode AArch64

QEMU version 7.2.0, static user-mode binary. Target:
`aarch64-unknown-linux-musl`, linked with `rust-lld`.

```bash
rustup target add aarch64-unknown-linux-musl
scripts/qemu-aarch64-test.sh
```

Result:

```
running 48 tests
test result: ok. 48 passed; 0 failed
test result: ok. 1 passed        # tests/examples.rs
```

On this target `cfg!(target_arch = "aarch64")` is true, so the execution
tests actually enter the trampoline and run:

- `mov x0, #42; ret` → `x0 = 42`, x1–x29 = 0.
- raw hex `400580d2c0035fd6` → `x0 = 42`.
- `fib.s` → `x0 = 55`.

QEMU user-mode validates the trampoline and Rust execution path. It does not
validate Android Binder/`:worker` lifecycle or Compose UI; the API 26 emulator
section covers those.

### Encoding cross-check

```bash
cargo build --manifest-path core/Cargo.toml --bin a64asm
python3 scripts/verify-encodings.py
```

Result:

```
270/270 encodings match LLVM
```

The reference is the NDK 28.2.13676358 `aarch64-linux-android21-clang` and
`llvm-objdump`.

### Kotlin/JVM tests

```bash
./gradlew :app:testDebugUnitTest --offline --rerun-tasks
```

Result: **5 tests pass** (`HighlightTest`): token-stream parsing, malformed-line
rejection, empty input, minimap line caps and long-file sampling.

## APK verification

The final artifacts were rebuilt from the current source tree with the release
signing environment loaded:

```bash
source ~/.a64studio-release.env
./gradlew :app:clean :app:assembleDebug :app:assembleRelease \
  --offline --no-parallel
```

Artifacts:

| Artifact | Size | SHA-256 |
|---|---:|---|
| `dist/a64studio-0.1.0-debug.apk` (also `app/build/outputs/apk/debug/app-debug.apk`) | 9,668,802 bytes | `9e3a6cabeb7da54559fa34756c3deda4363f576e62d8652507ce385d3674d556` |
| `dist/a64studio-0.1.0-release.apk` (also `app/build/outputs/apk/release/app-release.apk`) | 7,325,907 bytes | `6a4fe08a23145411b353816f56587ee81cfcc507f3796b51f827a463b4af548e` |

`dist/SHA256SUMS` contains the same two hashes. The `dist/` directory is
git-ignored; the APKs are local deliverables, not committed binaries.

The release APK is signed with a local development key:

```
Verifies
Verified using v1 scheme (JAR signing): false
Verified using v2 scheme (APK Signature Scheme v2): true
Number of signers: 1
V2 Signer: certificate DN: CN=A64 Studio, OU=Development, O=A64 Studio, L=Local, ST=Local, C=US
V2 Signer: key algorithm: RSA
V2 Signer: key size (bits): 2048
```

Both APKs contain:

```
lib/arm64-v8a/liba64core.so
lib/arm64-v8a/libandroidx.graphics.path.so
assets/examples/fib.s
assets/examples/hello.s
assets/examples/memory.s
assets/examples/neon.s
assets/examples/registers.s
```

The packaged native library is stripped by Gradle; its SHA-256 inside both
APKs is:

```
427bfb05a05c556c36836ee90e18f5514c10793f717761e74ee17aee50e92c0f
```

The release manifest requests no `android.permission.INTERNET`; the only
declared permission is the Android-generated
`dev.a64studio.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`.

The local signing key is **not** in the repository. It lives outside the
project and is only used to produce an installable local release. F-Droid
signs with its own key.

## Known verification gaps

- Physical re-test of guest `x10 = 0` (and x1–x29 generally) after the
  trampoline fix. The API 26 emulator instrumentation covers this, but the
  Pixel was unplugged before the rebuilt APK could be reinstalled.
- Testing on a 16 KiB-page ARM64 device and on non-Pixel ARM64 SoCs.
- Android versions other than API 26 on the ARM64 emulator; the Pixel evidence
  is Android 17.
- Long-running benchmark accuracy on thermal-throttled devices.
- Automated Compose UI interaction beyond the manual emulator run and the
  JVM highlighting tests.
