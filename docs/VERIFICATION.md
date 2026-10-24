# Verification

This document records what was actually tested, on which hardware, and what
was only tested in a fallback environment. It deliberately separates
real-device evidence from host/QEMU evidence.

## Real device

- Device: Google Pixel 8
- Android: 17
- ABI: `arm64-v8a`
- App: debug build (`dev.a64studio.debug`), later rebuilt with the final
  trampoline fix
- Connection: USB debugging, no root, no emulator

Commands used for the basic device checks:

```bash
adb devices
adb shell getprop ro.product.cpu.abi
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.a64studio.debug/dev.a64studio.MainActivity
```

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

The screenshots in `docs/screenshots/` were captured from the Pixel 8:

- `editor-fib.png` — Fibonacci source and completed run.
- `registers-fib.png` — register inspector after Fibonacci.
- `diagnostics.png` — invalid immediate, underline and diagnostic.
- `hex-patcher.png` — raw-hex disassembly/execution screen.
- `worker-timeout.png` — status after the infinite branch was killed.

### 7. Register-leak finding

The `registers-fib.png` capture also exposed a bug: guest `x10` contained a
host pointer (`0xd381a83dbf90`) instead of zero. The trampoline had used
`x10` as the register-file base and loaded guest `x10` too early. The
trampoline now loads `x10` last and uses `x30` for the trampoline pointer.
The Rust test suite asserts that x1–x29 are zero after `mov x0,#42; ret`, and
that test passes under QEMU. The fix is included in the rebuilt debug and
release APKs. A physical-device re-check of the `x10` value is still pending
because the Pixel was unplugged before the fix was made.

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
running 47 tests
test result: ok. 47 passed; 0 failed
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
running 47 tests
test result: ok. 47 passed; 0 failed
test result: ok. 1 passed        # tests/examples.rs
```

On this target `cfg!(target_arch = "aarch64")` is true, so the execution
tests actually enter the trampoline and run:

- `mov x0, #42; ret` → `x0 = 42`, x1–x29 = 0.
- raw hex `400580d2c0035fd6` → `x0 = 42`.
- `fib.s` → `x0 = 55`.

This is the fallback that validates the trampoline after the Pixel was
unplugged. It cannot validate Android Binder/`:worker` lifecycle or Compose
UI.

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
./gradlew :app:testDebugUnitTest
```

Result: 5 tests pass (`HighlightTest`): token-stream parsing, malformed-line
rejection, empty input, minimap line caps and long-file sampling.

### Android instrumentation build

```bash
./gradlew :app:assembleDebugAndroidTest
```

Result: `BUILD SUCCESSFUL`. The test APK compiles. It contains tests for:

- real `mov x0, #42; ret` execution;
- raw-hex execution;
- diagnostics with line/column;
- disassembly of `mov`/`ret`;
- worker watchdog termination of `b .`;
- project save/reopen;
- absence of `INTERNET`.

The connected run (`connectedDebugAndroidTest`) was not completed in this
workspace because the only ARM64 device was unplugged. The manual checks in
the first half of this document cover the same execution and timeout paths,
except save/reopen. Save/reopen is covered by the instrumentation test source,
but the final automated run is pending a device; the manual save/reopen flow
was not completed before the Pixel was unplugged.

## APK verification

Final rebuild after the trampoline fix:

```bash
./gradlew :app:assembleDebug :app:assembleRelease
```

Artifacts:

| Artifact | Size | SHA-256 |
|---|---:|---|
| `app/build/outputs/apk/debug/app-debug.apk` | 10 MB | `ef9deb87244646ca120937835d19ec87be85916f86a5b47ac7b1a34eabf02940` |
| `app/build/outputs/apk/release/app-release.apk` | 7.0 MB | `8fa3927d770333ddd311669ec777fe55d8932a862e6f85ebb00ff3657b65cab6` |

The release APK is signed with a local development key:

```
Verifies
Verified using v2 scheme (APK Signature Scheme v2): true
Number of signers: 1
```

Both APKs contain:

```
lib/arm64-v8a/liba64core.so
assets/examples/fib.s
assets/examples/hello.s
assets/examples/memory.s
assets/examples/neon.s
assets/examples/registers.s
```

The packaged native library is stripped by Gradle; its SHA-256 inside both
APKs is:

```
ff480e3f12257359677ffcf09b28702a66d6913a72d76b5c9e53ffc1ed7c28c1
```

The local signing key is **not** in the repository. It lives outside the
project and is only used to produce an installable local release.

## Known verification gaps

- Physical re-test of guest `x10 = 0` after the trampoline fix.
- Automated `connectedDebugAndroidTest` run (device was unplugged).
- Automated save/reopen on device (manual development check only).
- Testing on a 16 KiB-page ARM64 device and on non-Pixel ARM64 SoCs.
- Long-running benchmark accuracy on thermal-throttled devices.
