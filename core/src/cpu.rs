//! CPU and platform detection.
//!
//! Everything here is read-only and offline. The app uses it to explain what
//! the current device can and cannot do; it never fakes a missing feature.

use serde::Serialize;
use std::fs;

#[derive(Clone, Debug, Serialize)]
pub struct CpuInfo {
    pub arch: String,
    pub os: String,
    pub cores: usize,
    pub page_size: u64,
    pub hwcap: u64,
    pub hwcap2: u64,
    pub features: Vec<String>,
    pub implementer: Option<String>,
    pub part: Option<String>,
    pub hardware: Option<String>,
    pub model: Option<String>,
    pub execution_supported: bool,
    pub execution_note: String,
    pub wx_note: String,
    pub notes: Vec<String>,
}

fn read_cpuinfo() -> Vec<(String, String)> {
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

fn hwcap() -> u64 {
    #[cfg(unix)]
    unsafe {
        libc::getauxval(16) as u64
    }
    #[cfg(not(unix))]
    {
        0
    }
}

fn hwcap2() -> u64 {
    #[cfg(unix)]
    unsafe {
        libc::getauxval(26) as u64
    }
    #[cfg(not(unix))]
    {
        0
    }
}

fn decode_features(hwcap: u64, hwcap2: u64) -> Vec<String> {
    let bits = [
        (0, "fp"),
        (1, "asimd/neon"),
        (2, "evtstrm"),
        (3, "aes"),
        (4, "pmull"),
        (5, "sha1"),
        (6, "sha2"),
        (7, "crc32"),
        (8, "atomics"),
        (9, "fphp"),
        (10, "asimdhp"),
        (11, "cpuid"),
        (12, "asimdrdm"),
        (13, "jscvt"),
        (14, "fcma"),
        (15, "lrcpc"),
        (16, "dcpop"),
        (17, "sha3"),
        (18, "sm3"),
        (19, "sm4"),
        (20, "asimddp"),
        (21, "sha512"),
        (22, "sve"),
        (23, "asimdfhm"),
        (24, "dit"),
        (25, "uscat"),
        (26, "i8mm"),
        (27, "bti"),
        (28, "bf16"),
        (29, "dgh"),
        (30, "mte"),
        (31, "rndr"),
    ];
    let mut out = Vec::new();
    for (bit, name) in bits {
        if hwcap & (1u64 << bit) != 0 {
            out.push(name.to_string());
        }
    }
    let bits2 = [
        (1, "sve2"),
        (2, "sve2-bitperm"),
        (3, "sve-aes"),
        (4, "sve-pmull"),
        (5, "sve-sha3"),
        (6, "sve-sm4"),
        (7, "sve-i8mm"),
        (8, "sve-f32mm"),
        (9, "sve-f64mm"),
        (10, "sve-bf16"),
        (11, "sve-ebf16"),
        (12, "sve-sha512"),
    ];
    for (bit, name) in bits2 {
        if hwcap2 & (1u64 << bit) != 0 {
            out.push(name.to_string());
        }
    }
    out
}

pub fn cpu_info() -> CpuInfo {
    let info = read_cpuinfo();
    let find = |key: &str| info.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.clone());
    let mut features = Vec::new();
    for (k, v) in &info {
        if k.eq_ignore_ascii_case("Features") || k.eq_ignore_ascii_case("flags") {
            for f in v.split_whitespace() {
                if !features.iter().any(|x: &String| x == f) {
                    features.push(f.to_string());
                }
            }
        }
    }
    let h = hwcap();
    let h2 = hwcap2();
    for f in decode_features(h, h2) {
        if !features.iter().any(|x| x == &f) {
            features.push(f);
        }
    }
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let execution_supported = cfg!(target_arch = "aarch64");
    let execution_note = if execution_supported {
        "Native ARM64 execution is available. Code runs in a separate worker process with a watchdog timeout.".to_string()
    } else {
        "Native execution is unavailable on this host because it is not ARM64. A64 Studio does not emulate ARM instructions.".to_string()
    };
    let w_x_note = if cfg!(target_os = "android") {
        "Android W^X: the assembler maps code RX and .data RW separately. Self-modifying code is not supported.".to_string()
    } else {
        "This build uses separate RX code and RW data mappings; self-modifying code is not supported.".to_string()
    };
    let mut notes = Vec::new();
    if !execution_supported {
        notes.push("Build the APK for arm64-v8a and run it on a physical ARM64 Android device to execute assembly.".into());
    }
    if features.iter().any(|f| f == "sve" || f == "sve2") {
        notes.push("SVE/SVE2 is reported by the CPU, but this assembler only encodes the Advanced SIMD subset it documents.".into());
    }
    CpuInfo {
        arch: std::env::consts::ARCH.to_string(),
        os: std::env::consts::OS.to_string(),
        cores,
        page_size,
        hwcap: h,
        hwcap2: h2,
        features,
        implementer: find("CPU implementer"),
        part: find("CPU part"),
        hardware: find("Hardware"),
        model: find("model name").or_else(|| find("Processor")),
        execution_supported,
        execution_note,
        wx_note: w_x_note,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_something() {
        let c = cpu_info();
        assert!(!c.arch.is_empty());
        assert!(c.page_size > 0);
    }
}
