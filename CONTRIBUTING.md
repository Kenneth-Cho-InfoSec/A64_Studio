# Contributing to A64 Studio

Thanks for helping. A64 Studio is intentionally narrow: a real, offline
AArch64 IDE for Android. Please keep changes honest about what the platform
can do.

## Ground rules

- **No emulation.** Do not add QEMU, Unicorn, box64, Termux, a Linux
  userland, or a software CPU. On a non-ARM64 host the correct answer is an
  explicit "unsupported" result.
- **No fake features.** If an instruction, directive, CPU feature or
  inspector cannot be implemented, show a precise diagnostic or an
  "unsupported" message. Do not silently approximate.
- **No network.** The app must remain fully offline and must not request the
  `INTERNET` permission.
- **No root.** The app must work as an ordinary Android app.
- **No C/C++ app code.** Native logic belongs in Rust (`core/`). Kotlin and
  Jetpack Compose own the UI.

## Development setup

See the build section of [README.md](README.md). In short:

```bash
rustup target add aarch64-linux-android
./gradlew :app:assembleDebug
cd core && cargo test
```

The Gradle `cargoBuild` task cross-compiles `core/` for `arm64-v8a` and copies
`liba64core.so` into `app/src/main/jniLibs/arm64-v8a/`. Do not commit that
generated library; it is ignored by Git.

## Adding or changing an instruction

1. Add the encoder in `core/src/asm/encode/`.
2. Add the syntax/reference entry used by the Instruction Explorer.
3. Add a unit test in the encoder module.
4. Add the encoding to `scripts/verify-encodings.py` and run it against the
   NDK LLVM assembler:
   ```bash
   python3 scripts/verify-encodings.py
   ```
5. Run `cd core && cargo test`.

Do not add an instruction to the explorer unless the encoder actually
supports it.

## Testing

- Rust: `cd core && cargo test`
- Encoding cross-check: `python3 scripts/verify-encodings.py`
- Kotlin unit tests: `./gradlew :app:testDebugUnitTest`
- On-device: `./gradlew :app:connectedDebugAndroidTest` on an ARM64 device
- QEMU fallback: `scripts/qemu-aarch64-test.sh`

If you change the execution trampoline, run both the host tests and the QEMU
fallback. The QEMU path is not real hardware, so a physical ARM64 device test
is still preferred for a release.

## Code style

- Kotlin: 4 spaces, no wildcard imports, keep Compose state in
  `StudioViewModel` where practical.
- Rust: `cargo fmt` and `cargo clippy` where available. Keep the JNI surface
  in `core/src/jni_bridge.rs` and return JSON strings.
- Do not log guest data, project contents, or device identifiers.

## Commits and pull requests

- Keep commits focused and explain the user-visible effect.
- Include the tests you ran in the pull request description.
- If a change is device-specific, say which device/ABI/Android version you
  used.
- Do not include signing keys, `local.properties`, `app/build/`,
  `core/target/`, or generated `.so` files.

## Reporting bugs

Include:

- Device model, Android version, and `ro.product.cpu.abi`.
- App version and whether it is the debug or release build.
- The smallest assembly source or hex payload that reproduces the issue.
- `adb logcat` lines around the failure, if available.

Security issues: do not open a public issue with a working exploit. Contact
the maintainers privately first.
