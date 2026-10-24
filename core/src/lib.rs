//! A64 Studio native core.
//!
//! Modules:
//! * `asm`      — two-pass AArch64 assembler and encoders
//! * `disasm`   — pure-Rust AArch64 disassembler
//! * `exec`     — real native execution on ARM64 (never emulation)
//! * `cpu`      — CPU/feature/platform detection
//! * `bench`    — guest and host benchmarks
//! * `lexer`    — incremental syntax lexer
//! * `syntax`   — shared vocabulary and instruction reference
//! * `jni_bridge` — JSON JNI surface for the Kotlin app

pub mod asm;
pub mod bench;
pub mod cpu;
pub mod disasm;
pub mod exec;
pub mod jni_bridge;
pub mod lexer;
pub mod syntax;

/// Core version, also reported through JNI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
