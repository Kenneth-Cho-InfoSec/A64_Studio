//! Native AArch64 execution.
//!
//! This module only executes code on a real ARM64 CPU. On any other host it
//! returns an explicit "unsupported" result instead of emulating anything.
//!
//! Isolation model: the Android app calls this module from a dedicated
//! `:worker` process. A watchdog in that process kills the process when the
//! guest does not return in time. A guest crash therefore cannot take down the
//! editor or corrupt a saved project.

use crate::asm::{assemble, AssembleResult, RelocInfo};
use crate::disasm::{disassemble, DisasmInsn};
use serde::Serialize;

#[cfg(target_arch = "aarch64")]
use crate::asm::apply_relocs;
#[cfg(target_arch = "aarch64")]
use crate::disasm::hex_dump;
#[cfg(target_arch = "aarch64")]
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
pub struct RegisterView {
    pub name: String,
    pub value: u64,
    pub hex: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecResult {
    /// Whether this build can execute ARM64 code on the current CPU.
    pub supported: bool,
    pub ok: bool,
    pub reason: String,
    pub exit: String,
    pub elapsed_ns: u64,
    pub registers: Vec<RegisterView>,
    pub nzcv: u32,
    pub image_base: u64,
    pub image_size: u64,
    pub memory_hex: String,
    pub memory_len: u64,
    pub memory_truncated: bool,
    pub disassembly: Vec<DisasmInsn>,
    pub assembly: Option<AssembleResult>,
    pub timeout_ms: u64,
    pub note: String,
}

impl ExecResult {
    fn unsupported(reason: &str, assembly: Option<AssembleResult>, disasm: Vec<DisasmInsn>, timeout_ms: u64) -> Self {
        ExecResult {
            supported: false,
            ok: false,
            reason: reason.to_string(),
            exit: "unsupported".into(),
            elapsed_ns: 0,
            registers: Vec::new(),
            nzcv: 0,
            image_base: 0,
            image_size: 0,
            memory_hex: String::new(),
            memory_len: 0,
            memory_truncated: false,
            disassembly: disasm,
            assembly,
            timeout_ms,
            note: "A64 Studio never emulates ARM instructions. Run this project on an ARM64 Android device to execute it.".into(),
        }
    }
}

#[cfg(target_arch = "aarch64")]
fn register_views(regs: &[u64; 33]) -> Vec<RegisterView> {
    let mut out = Vec::with_capacity(33);
    for i in 0..31 {
        out.push(RegisterView {
            name: format!("x{}", i),
            value: regs[i],
            hex: format!("0x{:016x}", regs[i]),
            kind: "gpr".into(),
        });
    }
    out.push(RegisterView {
        name: "sp".into(),
        value: regs[31],
        hex: format!("0x{:016x}", regs[31]),
        kind: "sp".into(),
    });
    out.push(RegisterView {
        name: "nzcv".into(),
        value: regs[32],
        hex: format!("0x{:08x}", regs[32]),
        kind: "flags".into(),
    });
    out
}

#[cfg(target_arch = "aarch64")]
fn flags_text(nzcv: u64) -> String {
    let mut s = String::new();
    if nzcv & 0x8000_0000 != 0 { s.push('N'); } else { s.push('-'); }
    if nzcv & 0x4000_0000 != 0 { s.push('Z'); } else { s.push('-'); }
    if nzcv & 0x2000_0000 != 0 { s.push('C'); } else { s.push('-'); }
    if nzcv & 0x1000_0000 != 0 { s.push('V'); } else { s.push('-'); }
    s
}

/// Assemble and run a source file. Safe to call from any host; execution is
/// only attempted on AArch64.
pub fn run_source(source: &str, timeout_ms: u64) -> ExecResult {
    let asm = assemble(source);
    let disasm = disassemble(&asm.code, 0);
    if !asm.ok {
        return ExecResult {
            supported: cfg!(target_arch = "aarch64"),
            ok: false,
            reason: "assembly failed".into(),
            exit: "assembly-error".into(),
            elapsed_ns: 0,
            registers: Vec::new(),
            nzcv: 0,
            image_base: 0,
            image_size: 0,
            memory_hex: String::new(),
            memory_len: 0,
            memory_truncated: false,
            disassembly: disasm,
            assembly: Some(asm),
            timeout_ms,
            note: String::new(),
        };
    }
    run_assembled(asm, timeout_ms)
}

/// Run a raw little-endian instruction stream at `entry_offset` bytes.
pub fn run_raw(bytes: Vec<u8>, entry_offset: u64, timeout_ms: u64) -> ExecResult {
    if bytes.is_empty() {
        return ExecResult::unsupported("the hex payload is empty", None, Vec::new(), timeout_ms);
    }
    if entry_offset as usize >= bytes.len() || entry_offset % 4 != 0 {
        return ExecResult::unsupported("the entry offset must be 4-byte aligned and inside the payload", None, Vec::new(), timeout_ms);
    }
    let len = bytes.len();
    let asm = AssembleResult {
        ok: true,
        code: bytes,
        data: Vec::new(),
        entry: entry_offset,
        text_size: len as u64,
        data_size: 0,
        image_size: len as u64,
        symbols: Vec::new(),
        relocs: Vec::new(),
        instructions: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
        line_count: 0,
    };
    run_assembled(asm, timeout_ms)
}

fn run_assembled(asm: AssembleResult, timeout_ms: u64) -> ExecResult {
    let disasm = disassemble(&asm.code, 0);
    if cfg!(not(target_arch = "aarch64")) {
        return ExecResult::unsupported(
            "native execution is unavailable on this host: A64 Studio is running on a non-ARM64 CPU",
            Some(asm),
            disasm,
            timeout_ms,
        );
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        let _ = (asm, disasm);
        unreachable!()
    }
    #[cfg(target_arch = "aarch64")]
    {
        run_assembled_aarch64(asm, disasm, timeout_ms)
    }
}

#[cfg(target_arch = "aarch64")]
fn run_assembled_aarch64(asm: AssembleResult, disasm: Vec<DisasmInsn>, timeout_ms: u64) -> ExecResult {
    use std::ptr;

    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    if page == 0 {
        return ExecResult::unsupported("could not determine the page size", Some(asm), disasm, timeout_ms);
    }
    let text_len = asm.text_size as usize;
    let data_base = if asm.data_size > 0 {
        ((text_len as u64 + page - 1) / page) * page
    } else {
        text_len as u64
    };
    let image_len = (data_base + asm.data_size) as usize;
    let tramp_off = (text_len as u64 + 3) & !3;
    let map_len = (((image_len.max(tramp_off as usize + 4) as u64) + page - 1) / page) * page;

    let base = unsafe {
        libc::mmap(
            ptr::null_mut(),
            map_len as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if base == libc::MAP_FAILED {
        return ExecResult::unsupported("mmap failed while creating the code image", Some(asm), disasm, timeout_ms);
    }
    let image = base as *mut u8;
    unsafe {
        if !asm.code.is_empty() {
            ptr::copy_nonoverlapping(asm.code.as_ptr(), image, asm.code.len());
        }
        if !asm.data.is_empty() {
            ptr::copy_nonoverlapping(asm.data.as_ptr(), image.add(data_base as usize), asm.data.len());
        }
        // Trampoline: b <entry>. It is reached with `blr x30`, so x30 becomes
        // the return address and no general-purpose register is clobbered.
        let entry_addr = (base as u64).wrapping_add(asm.entry);
        let tramp_addr = (base as u64).wrapping_add(tramp_off);
        let delta = entry_addr as i64 - tramp_addr as i64;
        if delta % 4 != 0 || delta < -(1 << 27) || delta >= (1 << 27) {
            libc::munmap(base, map_len as usize);
            return ExecResult::unsupported("entry point is too far from the trampoline", Some(asm), disasm, timeout_ms);
        }
        let insn = 0x1400_0000u32 | (((delta >> 2) as u32) & 0x03FF_FFFF);
        ptr::write_unaligned(image.add(tramp_off as usize) as *mut u32, insn);

        let labels: HashMap<String, u64> = asm.symbols.iter().map(|s| (s.name.clone(), s.value)).collect();
        let mut image_slice = std::slice::from_raw_parts_mut(image, image_len);
        apply_relocs(&mut image_slice, &asm.relocs, base as u64, &labels);
    }

    // Code pages are RX; data pages stay RW so .data/.bss can be modified by
    // the guest. The assembler places .data on a page boundary.
    let code_prot_len = if asm.data_size > 0 { data_base } else { map_len };
    unsafe {
        if libc::mprotect(base, code_prot_len as usize, libc::PROT_READ | libc::PROT_EXEC) != 0 {
            libc::munmap(base, map_len as usize);
            return ExecResult::unsupported("mprotect(RX) failed; the platform may forbid executable app memory", Some(asm), disasm, timeout_ms);
        }
        if asm.data_size > 0 {
            let data_len = (((asm.data_size + page - 1) / page) * page) as usize;
            if libc::mprotect(image.add(data_base as usize) as *mut libc::c_void, data_len, libc::PROT_READ | libc::PROT_WRITE) != 0 {
                libc::munmap(base, map_len as usize);
                return ExecResult::unsupported("mprotect(RW) failed for .data", Some(asm), disasm, timeout_ms);
            }
        }
    }

    flush_icache(base as u64, map_len);

    // Private 64 KiB stack with a guard page. SP starts 16-byte aligned.
    let stack_guard = page as usize;
    let stack_size = 64 * 1024usize;
    let stack_map_len = stack_guard + stack_size;
    let stack_base = unsafe {
        libc::mmap(
            ptr::null_mut(),
            stack_map_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if stack_base == libc::MAP_FAILED {
        unsafe { libc::munmap(base, map_len as usize) };
        return ExecResult::unsupported("could not allocate the guest stack", Some(asm), disasm, timeout_ms);
    }
    unsafe {
        libc::mprotect(stack_base, stack_guard, libc::PROT_NONE);
    }
    let stack_top = ((stack_base as u64 + stack_map_len as u64) & !15u64) as u64;

    let mut regs = [0u64; 33];
    regs[31] = stack_top;
    let started = std::time::Instant::now();
    unsafe {
        a64studio_run(regs.as_mut_ptr(), stack_top as usize, (base as u64 + tramp_off) as usize);
    }
    let elapsed_ns = started.elapsed().as_nanos() as u64;

    // Copy the image after the run so .data edits are visible.
    let mut memory = vec![0u8; image_len];
    unsafe {
        ptr::copy_nonoverlapping(image as *const u8, memory.as_mut_ptr(), image_len);
    }
    let memory_truncated = memory.len() > 64 * 1024;
    if memory_truncated {
        memory.truncate(64 * 1024);
    }
    let memory_hex = hex_dump(&memory, base as u64);
    let memory_len = memory.len() as u64;

    unsafe {
        libc::munmap(stack_base, stack_map_len);
        libc::munmap(base, map_len as usize);
    }

    let nzcv = regs[32] as u32;
    let mut out = ExecResult {
        supported: true,
        ok: true,
        reason: String::new(),
        exit: format!("returned (NZCV {})", flags_text(regs[32])),
        elapsed_ns,
        registers: register_views(&regs),
        nzcv,
        image_base: base as u64,
        image_size: image_len as u64,
        memory_hex,
        memory_len,
        memory_truncated,
        disassembly: disasm,
        assembly: Some(asm),
        timeout_ms,
        note: "x30 is the host return address because the call uses the link register. The worker process enforces the timeout.".into(),
    };
    out
}

#[cfg(target_arch = "aarch64")]
fn flush_icache(addr: u64, len: u64) {
    use core::arch::asm;
    let line = 64u64;
    let mut p = addr & !(line - 1);
    let end = addr.saturating_add(len);
    unsafe {
        while p < end {
            asm!("dc cvau, {}", in(reg) p, options(nostack, preserves_flags));
            p += line;
        }
        asm!("dsb ish", options(nostack, preserves_flags));
        let mut q = addr & !(line - 1);
        while q < end {
            asm!("ic ivau, {}", in(reg) q, options(nostack, preserves_flags));
            q += line;
        }
        asm!("dsb ish", options(nostack, preserves_flags));
        asm!("isb", options(nostack, preserves_flags));
    }
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub static mut A64STUDIO_REGS: usize = 0;
#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub static mut A64STUDIO_HOST_SP: usize = 0;
#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub static mut A64STUDIO_TRAMP: usize = 0;

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
.text
.hidden A64STUDIO_REGS
.hidden A64STUDIO_HOST_SP
.hidden A64STUDIO_TRAMP
.global a64studio_run
.type a64studio_run,%function
// x0 = registers pointer, x1 = guest stack top, x2 = trampoline address
a64studio_run:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    stp x21, x22, [sp, #-16]!
    stp x23, x24, [sp, #-16]!
    stp x25, x26, [sp, #-16]!
    stp x27, x28, [sp, #-16]!
    // x18 is the platform register on Android and is callee-saved by the
    // native ABI. The guest may use it, so preserve the host value too.
    str x18, [sp, #-16]!
    mov x9, sp
    adrp x10, A64STUDIO_HOST_SP
    add x10, x10, :lo12:A64STUDIO_HOST_SP
    str x9, [x10]
    adrp x10, A64STUDIO_REGS
    add x10, x10, :lo12:A64STUDIO_REGS
    str x0, [x10]
    adrp x10, A64STUDIO_TRAMP
    add x10, x10, :lo12:A64STUDIO_TRAMP
    str x2, [x10]
    mov x10, x0
    mov sp, x1
    ldp x0, x1, [x10, #0]
    ldp x2, x3, [x10, #16]
    ldp x4, x5, [x10, #32]
    ldp x6, x7, [x10, #48]
    ldp x8, x9, [x10, #64]
    // Keep the base in x10 until every other guest register is loaded. x10
    // itself is loaded last, otherwise the trampoline would leak a host
    // pointer into the guest register file.
    ldp x11, x12, [x10, #88]
    ldp x13, x14, [x10, #104]
    ldp x15, x16, [x10, #120]
    ldp x17, x18, [x10, #136]
    ldp x19, x20, [x10, #152]
    ldp x21, x22, [x10, #168]
    ldp x23, x24, [x10, #184]
    ldp x25, x26, [x10, #200]
    ldp x27, x28, [x10, #216]
    ldr x29, [x10, #232]
    ldr x10, [x10, #80]
    // Enter through the trampoline with the link register as the return stub.
    // x30 is the documented host return address, so it is safe to use as a
    // scratch register for the static trampoline pointer here.
    adrp x30, A64STUDIO_TRAMP
    add x30, x30, :lo12:A64STUDIO_TRAMP
    ldr x30, [x30]
    blr x30
a64studio_return:
    // Save the two registers we need as scratch before using x10 as the
    // destination base. Below-SP memory is free on AArch64.
    sub sp, sp, #16
    stp x10, x16, [sp]
    adrp x10, A64STUDIO_REGS
    add x10, x10, :lo12:A64STUDIO_REGS
    ldr x10, [x10]
    stp x0, x1, [x10, #0]
    stp x2, x3, [x10, #16]
    stp x4, x5, [x10, #32]
    stp x6, x7, [x10, #48]
    stp x8, x9, [x10, #64]
    stp x11, x12, [x10, #88]
    stp x13, x14, [x10, #104]
    str x15, [x10, #120]
    str x17, [x10, #136]
    stp x18, x19, [x10, #144]
    stp x20, x21, [x10, #160]
    stp x22, x23, [x10, #176]
    stp x24, x25, [x10, #192]
    stp x26, x27, [x10, #208]
    stp x28, x29, [x10, #224]
    str x30, [x10, #240]
    add x11, sp, #16
    str x11, [x10, #248]
    mrs x11, nzcv
    str x11, [x10, #256]
    ldp x11, x12, [sp]
    str x11, [x10, #80]
    str x12, [x10, #128]
    add sp, sp, #16
    adrp x10, A64STUDIO_HOST_SP
    add x10, x10, :lo12:A64STUDIO_HOST_SP
    ldr x9, [x10]
    mov sp, x9
    // Restore in the exact reverse order of the pushes above. The stack
    // layout is [x18,pad], [x27,x28], [x25,x26], ..., [x19,x20],
    // [x29,x30] from low to high address.
    ldr x18, [sp], #16
    ldp x27, x28, [sp], #16
    ldp x25, x26, [sp], #16
    ldp x23, x24, [sp], #16
    ldp x21, x22, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
.size a64studio_run, .-a64studio_run
"#
);

#[cfg(target_arch = "aarch64")]
extern "C" {
    fn a64studio_run(regs: *mut u64, stack_top: usize, tramp: usize);
}

/// Disassemble a raw hex string. Returns a JSON-friendly structure via the
/// caller. This is useful on x86 hosts and for the Hex Patcher.
pub fn disassemble_hex(hex: &str, base: u64) -> Result<Vec<DisasmInsn>, String> {
    let bytes = parse_hex(hex)?;
    Ok(disassemble(&bytes, base))
}

pub fn parse_hex(hex: &str) -> Result<Vec<u8>, String> {
    let mut digits = String::new();
    for line in hex.lines() {
        let line = line.split("//").next().unwrap_or("");
        for token in line.split_whitespace() {
            let t = token.trim_start_matches("0x").trim_start_matches("0X");
            if t.chars().all(|c| c.is_ascii_hexdigit()) && !t.is_empty() {
                digits.push_str(t);
            } else if token.starts_with(';') || token.starts_with('#') {
                break;
            } else {
                return Err(format!("invalid hex token '{}'", token));
            }
        }
    }
    if digits.len() % 2 != 0 {
        return Err("hex payload has an odd number of digits".into());
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let bytes = digits.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let s = std::str::from_utf8(&bytes[i..i + 2]).map_err(|_| "invalid hex".to_string())?;
        out.push(u8::from_str_radix(s, 16).map_err(|_| "invalid hex".to_string())?);
        i += 2;
    }
    Ok(out)
}

/// Build a short report for the UI when relocations were applied.
pub fn relocation_report(relocs: &[RelocInfo]) -> String {
    if relocs.is_empty() {
        "no relocations".into()
    } else {
        format!("{} absolute data relocation(s)", relocs.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_42_executes_natively_or_is_explicitly_unsupported() {
        let r = run_source("mov x0, #42\nret\n", 1000);
        assert!(r.assembly.is_some());
        if cfg!(target_arch = "aarch64") {
            assert!(r.supported);
            assert!(r.ok);
            assert_eq!(r.registers[0].value, 42);
            // No host scratch value may leak into the guest register file.
            for i in 1..30 {
                assert_eq!(r.registers[i].value, 0, "x{} leaked a host value", i);
            }
        } else {
            assert!(!r.supported);
            assert!(!r.ok);
            assert!(r.reason.contains("non-ARM64"));
        }
    }

    #[test]
    fn raw_hex_mov_42_executes_natively_or_is_explicitly_unsupported() {
        let bytes = parse_hex("400580d2c0035fd6").unwrap();
        let r = run_raw(bytes, 0, 1000);
        if cfg!(target_arch = "aarch64") {
            assert!(r.supported);
            assert!(r.ok);
            assert_eq!(r.registers[0].value, 42);
        } else {
            assert!(!r.supported);
            assert!(!r.ok);
        }
    }

    #[test]
    fn fib_example_returns_55_on_aarch64() {
        let r = run_source(include_str!("../../app/src/main/assets/examples/fib.s"), 1000);
        assert!(r.assembly.is_some());
        if cfg!(target_arch = "aarch64") {
            assert!(r.ok);
            assert_eq!(r.registers[0].value, 55);
        }
    }

    #[test]
    fn parses_hex() {
        assert_eq!(parse_hex("d2 80 05 40\n").unwrap(), vec![0xd2, 0x80, 0x05, 0x40]);
        assert!(parse_hex("abc").is_err());
    }
}
