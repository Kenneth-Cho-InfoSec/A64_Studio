# Dependencies and licenses

A64 Studio is licensed under **GPL-3.0-or-later**. This report covers the
runtime and test dependencies that are distributed with or used to build the
app. Build tools (Gradle, Android Gradle Plugin, Kotlin compiler, Rust
toolchain) are not redistributed in the APK.

## Android / Kotlin runtime

| Dependency | Version | License |
|---|---|---|
| `androidx.core:core-ktx` | 1.15.0 | Apache-2.0 |
| `androidx.activity:activity-compose` | 1.9.3 | Apache-2.0 |
| `androidx.compose:compose-bom` | 2024.10.01 | Apache-2.0 |
| `androidx.compose.ui:ui` | BOM | Apache-2.0 |
| `androidx.compose.ui:ui-graphics` | BOM | Apache-2.0 |
| `androidx.compose.ui:ui-tooling-preview` | BOM | Apache-2.0 |
| `androidx.compose.material3:material3` | BOM | Apache-2.0 |
| `androidx.compose.foundation:foundation` | BOM | Apache-2.0 |
| `androidx.lifecycle:lifecycle-runtime-ktx` | 2.8.6 | Apache-2.0 |
| `androidx.lifecycle:lifecycle-viewmodel-compose` | 2.8.6 | Apache-2.0 |
| `androidx.lifecycle:lifecycle-viewmodel-ktx` | 2.8.6 | Apache-2.0 |
| `org.jetbrains.kotlinx:kotlinx-coroutines-android` | 1.9.0 | Apache-2.0 |
| Kotlin standard library | 2.0.21 | Apache-2.0 |

AndroidX and Jetpack Compose are Apache License 2.0. The Kotlin standard
library and kotlinx.coroutines are Apache License 2.0.

## Test-only

| Dependency | Version | License |
|---|---|---|
| `junit:junit` | 4.13.2 | EPL-1.0 |
| `androidx.test.ext:junit` | 1.2.1 | Apache-2.0 |
| `androidx.test:runner` | 1.6.2 | Apache-2.0 |
| `androidx.test:rules` | 1.6.1 | Apache-2.0 |

These are not part of the release APK.

## Rust core

The Rust dependencies are pinned by `core/Cargo.lock`. The table below is
generated from the locked packages. Most are dual-licensed MIT/Apache-2.0;
`yaxpeax-arm` and `yaxpeax-arch` are 0BSD.

| Crate | Version | License |
|---|---|---|
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT |
| `bitvec` | 1.1.1 | MIT |
| `bytes` | 1.12.1 | MIT |
| `cesu8` | 1.1.0 | Apache-2.0/MIT |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 |
| `combine` | 4.6.9 | MIT |
| `funty` | 2.0.0 | MIT |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 |
| `jni` | 0.21.1 | MIT/Apache-2.0 |
| `jni-sys` | 0.3.1 | MIT OR Apache-2.0 |
| `jni-sys` | 0.4.1 | MIT OR Apache-2.0 |
| `jni-sys-macros` | 0.4.1 | MIT OR Apache-2.0 |
| `libc` | 0.2.191 | MIT OR Apache-2.0 |
| `log` | 0.4.34 | MIT OR Apache-2.0 |
| `memchr` | 2.8.3 | Unlicense OR MIT |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | MIT OR Apache-2.0 |
| `radium` | 0.7.0 | MIT |
| `same-file` | 1.0.6 | Unlicense/MIT |
| `serde` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 |
| `syn` | 2.0.119 | MIT OR Apache-2.0 |
| `syn` | 3.0.9 | MIT OR Apache-2.0 |
| `tap` | 1.0.1 | MIT |
| `thiserror` | 1.0.69 | MIT OR Apache-2.0 |
| `thiserror-impl` | 1.0.69 | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| `walkdir` | 2.5.0 | Unlicense/MIT |
| `winapi-util` | 0.1.11 | Unlicense OR MIT |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 |
| `windows-sys` | 0.45.0 | MIT OR Apache-2.0 |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 |
| `windows-targets` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_aarch64_gnullvm` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_aarch64_msvc` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_i686_gnu` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_i686_msvc` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_x86_64_gnu` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_x86_64_gnullvm` | 0.42.2 | MIT OR Apache-2.0 |
| `windows_x86_64_msvc` | 0.42.2 | MIT OR Apache-2.0 |
| `wyz` | 0.5.1 | MIT |
| `yaxpeax-arch` | 0.3.3 | 0BSD |
| `yaxpeax-arm` | 0.3.1 | 0BSD |
| `zmij` | 1.0.23 | MIT |

### Direct Rust dependencies

| Crate | Version | License | Purpose |
|---|---|---|---|
| `jni` | 0.21.1 | MIT OR Apache-2.0 | JNI bindings |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | `mmap`, `mprotect`, `sysconf` |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | Result serialization |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | JSON JNI surface |
| `yaxpeax-arm` | 0.3.1 | 0BSD | AArch64 disassembler |
| `yaxpeax-arch` | 0.3.3 | 0BSD | Disassembler traits |

## License texts

- Apache-2.0: <https://www.apache.org/licenses/LICENSE-2.0>
- MIT: <https://opensource.org/license/mit>
- 0BSD: <https://opensource.org/license/0bsd>
- EPL-1.0: <https://www.eclipse.org/legal/epl-v10.html>
- Unlicense: <https://unlicense.org/>
- Unicode-3.0: <https://www.unicode.org/license.txt>

The full GPL-3.0 text is in [../LICENSE](../LICENSE). If you redistribute a
modified version, keep the copyright and license notices of the dependencies
above and make the corresponding source available under the GPL.

## Reproducibility

- `core/Cargo.lock` pins the Rust dependency graph.
- `app/gradle.lockfile` pins the Gradle dependency graph.
- `gradle/wrapper/gradle-wrapper.properties` pins Gradle 8.9.
- `scripts/verify-encodings.py` pins the encoding cross-check to NDK
  `28.2.13676358` and its LLVM tools.
