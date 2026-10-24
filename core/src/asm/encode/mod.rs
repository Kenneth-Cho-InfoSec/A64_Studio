//! AArch64 instruction encoder.
//!
//! This is a hand-written, table-assisted encoder for a documented subset of
//! GNU-style AArch64 assembly. Every encoding family implemented here is
//! cross-checked against LLVM's AArch64 assembler in `tests/llvm_vectors.rs`
//! and the verification script in `scripts/verify-encodings.sh`.
//!
//! Design notes:
//! * Pseudo-instructions never change instruction size: aliases expand to one
//!   32-bit instruction, and large `mov`/`ldr =` immediates use a literal pool
//!   entry appended to `.text` by the assembler.
//! * No instruction is silently approximated. Anything the encoder cannot
//!   represent produces an error with the offending operand.

mod dp;
mod mem;
mod simd;
mod sys;

use crate::syntax::{condition_code, parse_gpr, parse_vreg, Gpr, VReg};
use std::fmt;

#[derive(Debug, Clone)]
pub struct EncError {
    pub message: String,
    /// Operand index the error refers to, if known.
    pub operand: Option<usize>,
}

impl EncError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), operand: None }
    }
    pub fn at(msg: impl Into<String>, op: usize) -> Self {
        Self { message: msg.into(), operand: Some(op) }
    }
}

impl fmt::Display for EncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

pub type EResult<T> = Result<T, EncError>;

pub struct Ctx<'a> {
    pub pc: u64,
    /// Evaluate an assembler expression (without a leading '#').
    pub eval: &'a dyn Fn(&str) -> Result<i64, String>,
    /// Offset of the literal-pool slot for a `ldr =expr` pseudo-instruction.
    pub pool_offset: Option<u64>,
    /// Allocator for literal pool slots, used by SIMD pseudo forms.
    pub section_base: u64,
}

impl<'a> Ctx<'a> {
    pub fn imm(&self, s: &str, op: usize) -> EResult<i64> {
        let t = s.trim();
        let t = t.strip_prefix('#').unwrap_or(t).trim();
        if t.is_empty() {
            return Err(EncError::at("expected an immediate", op));
        }
        if let Some(v) = parse_int_literal(t) {
            return Ok(v);
        }
        (self.eval)(t).map_err(|e| EncError::at(e, op))
    }
    pub fn imm_u(&self, s: &str, op: usize, bits: u32) -> EResult<u64> {
        let v = self.imm(s, op)?;
        let max: i64 = if bits >= 63 { i64::MAX } else { (1i64 << bits) - 1 };
        if v < 0 {
            let wrapped = v as u64;
            if bits < 64 && wrapped > (max as u64) {
                return Err(EncError::at(format!("immediate {} does not fit in {} bits", v, bits), op));
            }
            Ok(wrapped)
        } else if v > max {
            Err(EncError::at(format!("immediate {} does not fit in {} bits", v, bits), op))
        } else {
            Ok(v as u64)
        }
    }
    pub fn sym_offset(&self, name: &str) -> Option<i64> {
        (self.eval)(name).ok()
    }
}

/// Parse a plain integer literal (decimal, hex, octal or binary), allowing
/// underscores, an optional sign and a leading '#'.
pub fn parse_int_literal(s: &str) -> Option<i64> {
    let t = s.trim();
    let t = t.strip_prefix('#').unwrap_or(t).trim();
    let (neg, body) = match t.strip_prefix('-') {
        Some(r) => (true, r.trim()),
        None => (false, t.strip_prefix('+').unwrap_or(t).trim()),
    };
    if body.is_empty() {
        return None;
    }
    let clean: String = body.chars().filter(|c| *c != '_').collect();
    let (radix, digits) = if let Some(r) = clean.strip_prefix("0x").or_else(|| clean.strip_prefix("0X")) {
        (16, r)
    } else if let Some(r) = clean.strip_prefix("0b").or_else(|| clean.strip_prefix("0B")) {
        (2, r)
    } else if let Some(r) = clean.strip_prefix("0o").or_else(|| clean.strip_prefix("0O")) {
        (8, r)
    } else {
        (10, clean.as_str())
    };
    if digits.is_empty() {
        return None;
    }
    let v = i128::from_str_radix(digits, radix).ok()?;
    let v = if neg { -v } else { v };
    if v < i64::MIN as i128 || v > u64::MAX as i128 {
        return None;
    }
    Some(v as i64)
}

/// Split an operand list on top-level commas, respecting brackets and braces.
pub fn split_operands(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut in_str = false;
    let mut escaped = false;
    for c in s.chars() {
        if in_str {
            cur.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                cur.push(c);
            }
            '[' | '{' | '(' => {
                depth += 1;
                cur.push(c);
            }
            ']' | '}' | ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Merge shift/extend suffix operands into the preceding operand.
pub fn merge_shift_operands(ops: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for op in ops {
        let first = op.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
        let is_suffix = matches!(
            first.as_str(),
            "lsl" | "lsr" | "asr" | "ror" | "uxtb" | "uxth" | "uxtw" | "uxtx"
                | "sxtb" | "sxth" | "sxtw" | "sxtx" | "mul" | "vl"
        );
        if is_suffix && !out.is_empty() {
            let last = out.last_mut().unwrap();
            last.push(' ');
            last.push_str(&op);
        } else {
            out.push(op);
        }
    }
    out
}

pub fn gpr(s: &str, op: usize) -> EResult<Gpr> {
    let s = s.trim();
    parse_gpr(s).ok_or_else(|| EncError::at(format!("expected a general-purpose register, found '{}'", s), op))
}

pub fn vreg(s: &str, op: usize) -> EResult<(VReg, Option<u8>)> {
    let s = s.trim();
    parse_vreg(s).ok_or_else(|| EncError::at(format!("expected a SIMD/vector register, found '{}'", s), op))
}

/// Strip a shift suffix such as "lsl #3" or "uxtw #2".
pub fn split_suffix(s: &str) -> (&str, Option<(&str, Option<i64>)>) {
    let s = s.trim();
    // Find a top-level whitespace-separated suffix.
    let mut parts = s.splitn(2, char::is_whitespace);
    let core = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();
    if rest.is_empty() {
        return (core, None);
    }
    let mut rp = rest.split_whitespace();
    let kind = rp.next().unwrap_or("");
    let amount = rp.next().and_then(|a| {
        let a = a.trim_start_matches('#');
        a.parse::<i64>().ok()
    });
    (core, Some((kind, amount)))
}

pub fn parse_shift(s: &str) -> Option<(u32, i64)> {
    let (kind, amt) = split_suffix(s).1?;
    let sh = match kind.to_ascii_lowercase().as_str() {
        "lsl" => 0,
        "lsr" => 1,
        "asr" => 2,
        "ror" => 3,
        _ => return None,
    };
    Some((sh, amt.unwrap_or(0)))
}

pub fn parse_extend(s: &str) -> Option<(u32, i64)> {
    let (kind, amt) = split_suffix(s).1?;
    let op = match kind.to_ascii_lowercase().as_str() {
        "uxtb" => 0,
        "uxth" => 1,
        "uxtw" => 2,
        "uxtx" => 3,
        "sxtb" => 4,
        "sxth" => 5,
        "sxtw" => 6,
        "sxtx" => 7,
        _ => return None,
    };
    Some((op, amt.unwrap_or(0)))
}

pub fn cond(s: &str, op: usize) -> EResult<u8> {
    condition_code(s.trim()).ok_or_else(|| EncError::at(format!("expected a condition code, found '{}'", s), op))
}

pub fn is_zero_reg(g: Gpr) -> bool {
    g.num == 31 && !g.is_sp
}

pub fn is_sp(g: Gpr) -> bool {
    g.num == 31 && g.is_sp
}

pub fn check_same_width(a: Gpr, b: Gpr, op: usize) -> EResult<()> {
    if a.width != b.width {
        return Err(EncError::at(
            format!("register width mismatch: {} and {}", a.text(), b.text()),
            op,
        ));
    }
    Ok(())
}

pub fn require_width(g: Gpr, width: u8, op: usize) -> EResult<()> {
    if g.width != width {
        return Err(EncError::at(format!("expected a {}-bit register, found {}", width, g.text()), op));
    }
    Ok(())
}

/// Encode a bitmask (logical) immediate. Returns (N, immr, imms).
pub fn encode_logical_imm(value: u64, width: u32) -> Option<(u32, u32, u32)> {
    let mask: u64 = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
    let v = value & mask;
    if v == 0 {
        return None;
    }
    let mut esize = 2u32;
    while esize <= width {
        let emask: u64 = if esize == 64 { u64::MAX } else { (1u64 << esize) - 1 };
        let pattern = v & emask;
        let mut rep: u64 = 0;
        let mut i = 0;
        while i < width / esize {
            rep = rep.wrapping_shl(esize) | pattern;
            i += 1;
        }
        if rep == v {
            let ones = pattern.count_ones();
            if ones == 0 {
                return None;
            }
            // Find a rotation that moves the run of ones to the low bits.
            for r in 0..esize {
                let rot = rotr_n(pattern, r, esize);
                let expect = if ones == 64 { u64::MAX } else { (1u64 << ones) - 1 };
                if rot == expect {
                    let n = if esize == 64 { 1 } else { 0 };
                    let imms = if esize == 64 {
                        (ones - 1) & 0x3f
                    } else {
                        (((!(esize - 1)) << 1) | (ones - 1)) & 0x3f
                    };
                    return Some((n, r, imms));
                }
            }
        }
        esize <<= 1;
    }
    None
}

fn rotr_n(v: u64, r: u32, width: u32) -> u64 {
    let mask: u64 = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
    let v = v & mask;
    if r == 0 {
        return v;
    }
    ((v >> r) | (v << (width - r))) & mask
}

pub fn is_logical_imm_encodable(v: i64, width: u32) -> bool {
    encode_logical_imm(v as u64, width).is_some()
}

/// Whether `mov Rd, #value` fits in one instruction.
pub fn mov_imm_encodable(v: i64, width: u8) -> bool {
    let w = width as u32;
    let mask: u64 = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
    let uv = (v as u64) & mask;
    movz_imm(uv, w).is_some() || movn_imm(uv, w).is_some() || encode_logical_imm(uv, w).is_some()
}

/// Try to encode a 64-bit immediate as a single MOVZ/MOVN (optionally shifted).
pub fn movz_imm(v: u64, width: u32) -> Option<(u32, u16)> {
    let mask: u64 = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
    let v = v & mask;
    for hw in 0..(width / 16) {
        let shifted = (v >> (hw * 16)) & 0xffff;
        let rest = v & !(0xffffu64 << (hw * 16));
        if rest == 0 {
            return Some((hw, shifted as u16));
        }
    }
    None
}

pub fn movn_imm(v: u64, width: u32) -> Option<(u32, u16)> {
    let mask: u64 = if width >= 64 { u64::MAX } else { (1u64 << width) - 1 };
    let inv = (!v) & mask;
    movz_imm(inv, width)
}

pub fn insn_addsub_imm(sf: bool, sub: bool, setflags: bool, shift12: bool, imm: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1100_0000u32;
    if sf { w |= 1 << 31; }
    if sub { w |= 1 << 30; }
    if setflags { w |= 1 << 29; }
    if shift12 { w |= 1 << 22; }
    w |= (imm & 0xfff) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_addsub_reg(sf: bool, sub: bool, setflags: bool, shift: u32, imm6: u32, rm: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x0B00_0000u32;
    if sf { w |= 1 << 31; }
    if sub { w |= 1 << 30; }
    if setflags { w |= 1 << 29; }
    w |= (shift & 3) << 22;
    w |= (rm & 31) << 16;
    w |= (imm6 & 63) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_addsub_ext(sf: bool, sub: bool, setflags: bool, rm: u32, option: u32, imm3: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x0B20_0000u32;
    if sf { w |= 1 << 31; }
    if sub { w |= 1 << 30; }
    if setflags { w |= 1 << 29; }
    w |= (rm & 31) << 16;
    w |= (option & 7) << 13;
    w |= (imm3 & 7) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_logical_imm(sf: bool, opc: u32, n: u32, immr: u32, imms: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1200_0000u32;
    if sf { w |= 1 << 31; }
    w |= (opc & 3) << 29;
    w |= (n & 1) << 22;
    w |= (immr & 63) << 16;
    w |= (imms & 63) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_logical_reg(sf: bool, opc: u32, shift: u32, invert: bool, rm: u32, imm6: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x0A00_0000u32;
    if sf { w |= 1 << 31; }
    w |= (opc & 3) << 29;
    w |= (shift & 3) << 22;
    if invert { w |= 1 << 21; }
    w |= (rm & 31) << 16;
    w |= (imm6 & 63) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_movwide(sf: bool, opc: u32, hw: u32, imm16: u32, rd: u32) -> u32 {
    let mut w = 0x1280_0000u32;
    if sf { w |= 1 << 31; }
    w |= (opc & 3) << 29;
    w |= (hw & 3) << 21;
    w |= (imm16 & 0xffff) << 5;
    w |= rd & 31;
    w
}

pub fn insn_bitfield(sf: bool, opc: u32, immr: u32, imms: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1300_0000u32;
    if sf { w |= 1 << 31; }
    w |= (opc & 3) << 29;
    if sf { w |= 1 << 22; }
    w |= (immr & 63) << 16;
    w |= (imms & 63) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_condsel(sf: bool, op2: u32, o2: u32, rm: u32, cond: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1A80_0000u32;
    if sf { w |= 1 << 31; }
    w |= (op2 & 1) << 30;
    w |= (rm & 31) << 16;
    w |= (cond & 15) << 12;
    w |= (o2 & 1) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_dp2(sf: bool, opcode: u32, rm: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1AC0_0000u32;
    if sf { w |= 1 << 31; }
    w |= (rm & 31) << 16;
    w |= (opcode & 0x3f) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_dp1(sf: bool, opcode2: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x5AC0_0000u32;
    if sf { w |= 1 << 31; }
    w |= (opcode2 & 0x3f) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_madd(sf: bool, o0: bool, rm: u32, ra: u32, rn: u32, rd: u32) -> u32 {
    let mut w = 0x1B00_0000u32;
    if sf { w |= 1 << 31; }
    w |= (rm & 31) << 16;
    if o0 { w |= 1 << 15; }
    w |= (ra & 31) << 10;
    w |= (rn & 31) << 5;
    w |= rd & 31;
    w
}

pub fn insn_branch(link: bool, imm26: i64) -> u32 {
    let op = if link { 0x9400_0000u32 } else { 0x1400_0000u32 };
    op | ((imm26 as u32) & 0x03ff_ffff)
}

pub fn insn_bcond(imm19: i64, cond: u32) -> u32 {
    0x5400_0000 | (((imm19 as u32) & 0x7ffff) << 5) | (cond & 15)
}

pub fn insn_cbz(sf: bool, nonzero: bool, imm19: i64, rt: u32) -> u32 {
    let mut w = if nonzero { 0x3500_0000u32 } else { 0x3400_0000u32 };
    if sf { w |= 1 << 31; }
    w | (((imm19 as u32) & 0x7ffff) << 5) | (rt & 31)
}

pub fn insn_tbz(b5: bool, nonzero: bool, b40: u32, imm14: i64, rt: u32) -> u32 {
    let mut w = if nonzero { 0x3700_0000u32 } else { 0x3600_0000u32 };
    if b5 { w |= 1 << 31; }
    w |= (b40 & 31) << 19;
    w | (((imm14 as u32) & 0x3fff) << 5) | (rt & 31)
}

pub fn insn_adr(rd: u32, imm: i64) -> u32 {
    0x1000_0000 | (((imm as u32) & 0x3) << 29) | ((((imm >> 2) as u32) & 0x7ffff) << 5) | (rd & 31)
}

pub fn insn_adrp(rd: u32, page_delta: i64) -> u32 {
    0x9000_0000 | (((page_delta as u32) & 0x3) << 29) | ((((page_delta >> 2) as u32) & 0x7ffff) << 5) | (rd & 31)
}

/// Main dispatch.
pub fn encode(mnemonic: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    let m = mnemonic.to_ascii_lowercase();
    if m.starts_with("b.") {
        let c = condition_code(&m[2..]).ok_or_else(|| EncError::new(format!("unknown condition in '{}'", mnemonic)))?;
        if ops.len() != 1 {
            return Err(EncError::new("conditional branch takes exactly one target"));
        }
        let target = (ctx.eval)(ops[0].trim()).map_err(|e| EncError::at(e, 0))?;
        let delta = target - ctx.pc as i64;
        if delta % 4 != 0 {
            return Err(EncError::at("branch target is not 4-byte aligned", 0));
        }
        let imm = delta / 4;
        if !(-(1 << 18)..(1 << 18)).contains(&imm) {
            return Err(EncError::at("branch target is out of range (±1 MiB)", 0));
        }
        return Ok(vec![insn_bcond(imm, c as u32)]);
    }
    // Vector operands are routed to the SIMD encoder even for mnemonics
    // shared with scalar data-processing (add, sub, and, ...).
    if simd::handles(&m) {
        let vec = ops.first().map(|o| {
            let t = o.trim();
            t.starts_with('{') || parse_vreg(t).is_some()
        }).unwrap_or(false);
        if vec {
            return simd::encode(&m, ops, ctx);
        }
    }
    match m.as_str() {
        // Data processing / aliases.
        "mov" | "movz" | "movn" | "movk" | "add" | "adds" | "sub" | "subs" | "cmp" | "cmn"
        | "neg" | "negs" | "and" | "orr" | "eor" | "ands" | "bic" | "orn" | "eon" | "bics"
        | "mvn" | "tst" | "lsl" | "lsr" | "asr" | "ror" | "lslv" | "lsrv" | "asrv" | "rorv"
        | "ubfm" | "sbfm" | "bfm" | "ubfx" | "sbfx" | "bfi" | "bfxil" | "sbfiz" | "ubfiz"
        | "sxtb" | "sxth" | "sxtw" | "uxtb" | "uxth" | "uxtw" | "rev" | "rev16" | "rev32"
        | "rbit" | "clz" | "cls" | "csel" | "csinc" | "csinv" | "csneg" | "cset" | "csetm"
        | "ccmp" | "ccmn" | "mul" | "madd" | "msub" | "mneg" | "smull" | "umull" | "smaddl"
        | "umaddl" | "smulh" | "umulh" | "sdiv" | "udiv" | "adr" | "adrp" | "crc32b"
        | "crc32h" | "crc32w" | "crc32x" | "crc32cb" | "crc32ch" | "crc32cw" | "crc32cx" => {
            dp::encode(&m, ops, ctx)
        }
        "ldr" | "str" | "ldrb" | "strb" | "ldrh" | "strh" | "ldrsb" | "ldrsh" | "ldrsw"
        | "ldp" | "stp" | "ldur" | "stur" | "ldurb" | "sturb" | "ldurh" | "sturh" | "ldursb"
        | "ldursh" | "ldursw" | "ldar" | "stlr" | "ldxr" | "stxr" | "ldaxr" | "stlxr" | "prfm" => {
            mem::encode(&m, ops, ctx)
        }
        "nop" | "hint" | "yield" | "wfe" | "wfi" | "sev" | "sevl" | "bti" | "svc" | "brk"
        | "hlt" | "dmb" | "dsb" | "isb" | "mrs" | "msr" | "paciasp" | "autiasp" | "pacibsp"
        | "autibsp" | "eret" | "drps" | "clrex" | "udf" => sys::encode(&m, ops, ctx),
        "b" | "bl" | "br" | "blr" | "ret" | "cbz" | "cbnz" | "tbz" | "tbnz" => {
            encode_branch(&m, ops, ctx)
        }
        _ if simd::handles(&m) => simd::encode(&m, ops, ctx),
        "sve" | "sve2" | "sme" => Err(EncError::new(
            "SVE/SME instructions are not implemented by this assembler build. The CPU explorer can report whether your device supports them.",
        )),
        _ => Err(EncError::new(format!(
            "instruction '{}' is not supported by the A64 Studio assembler",
            mnemonic
        ))),
    }
}

fn encode_branch(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    match m {
        "b" | "bl" => {
            if ops.len() != 1 {
                return Err(EncError::new(format!("'{}' takes one label or address", m)));
            }
            let target = (ctx.eval)(ops[0].trim()).map_err(|e| EncError::at(e, 0))?;
            let delta = target - ctx.pc as i64;
            if delta % 4 != 0 {
                return Err(EncError::at("branch target is not 4-byte aligned", 0));
            }
            let imm = delta / 4;
            if !(-(1 << 25)..(1 << 25)).contains(&imm) {
                return Err(EncError::at("branch target is out of range (±128 MiB)", 0));
            }
            Ok(vec![insn_branch(m == "bl", imm)])
        }
        "br" | "blr" => {
            if ops.len() != 1 {
                return Err(EncError::new(format!("'{}' takes one register", m)));
            }
            let r = gpr(&ops[0], 0)?;
            require_width(r, 64, 0)?;
            let base = if m == "br" { 0xD61F_0000u32 } else { 0xD63F_0000 };
            Ok(vec![base | ((r.num as u32) << 5)])
        }
        "ret" => {
            let r = if ops.is_empty() {
                Gpr { num: 30, width: 64, is_sp: false }
            } else if ops.len() == 1 {
                gpr(&ops[0], 0)?
            } else {
                return Err(EncError::new("'ret' takes at most one register"));
            };
            require_width(r, 64, 0)?;
            Ok(vec![0xD65F_0000 | ((r.num as u32) << 5)])
        }
        "cbz" | "cbnz" => {
            if ops.len() != 2 {
                return Err(EncError::new(format!("'{}' takes a register and a label", m)));
            }
            let r = gpr(&ops[0], 0)?;
            let target = (ctx.eval)(ops[1].trim()).map_err(|e| EncError::at(e, 1))?;
            let delta = target - ctx.pc as i64;
            if delta % 4 != 0 {
                return Err(EncError::at("branch target is not 4-byte aligned", 1));
            }
            let imm = delta / 4;
            if !(-(1 << 18)..(1 << 18)).contains(&imm) {
                return Err(EncError::at("branch target is out of range (±1 MiB)", 1));
            }
            Ok(vec![insn_cbz(r.width == 64, m == "cbnz", imm, r.num as u32)])
        }
        "tbz" | "tbnz" => {
            if ops.len() != 3 {
                return Err(EncError::new(format!("'{}' takes a register, a bit number and a label", m)));
            }
            let r = gpr(&ops[0], 0)?;
            let bit = ctx.imm_u(&ops[1], 1, 6)?;
            if r.width == 32 && bit > 31 {
                return Err(EncError::at("bit number out of range for a 32-bit register", 1));
            }
            let target = (ctx.eval)(ops[2].trim()).map_err(|e| EncError::at(e, 2))?;
            let delta = target - ctx.pc as i64;
            if delta % 4 != 0 {
                return Err(EncError::at("branch target is not 4-byte aligned", 2));
            }
            let imm = delta / 4;
            if !(-(1 << 13)..(1 << 13)).contains(&imm) {
                return Err(EncError::at("branch target is out of range (±32 KiB)", 2));
            }
            Ok(vec![insn_tbz(bit >= 32, m == "tbnz", (bit & 31) as u32, imm, r.num as u32)])
        }
        _ => Err(EncError::new(format!("unsupported branch '{}'", m))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_eval(_: &str) -> Result<i64, String> {
        Err("no symbols".into())
    }
    fn ctx() -> Ctx<'static> {
        Ctx { pc: 0, eval: &no_eval, pool_offset: None, section_base: 0 }
    }

    #[test]
    fn logical_immediates() {
        assert_eq!(encode_logical_imm(0xff, 64), Some((1, 0, 7)));
        assert_eq!(encode_logical_imm(0xff, 32), Some((0, 0, 7)));
        assert_eq!(encode_logical_imm(0xffff_0000_ffff_0000, 64), Some((0, 16, 15)));
        assert_eq!(encode_logical_imm(0, 64), None);
        // 0x5555... is the 2-bit pattern 01 repeated, which is encodable.
        assert_eq!(encode_logical_imm(0x5555_5555_5555_5555, 64), Some((0, 0, 60)));
        assert_eq!(encode_logical_imm(0x1234_5678_9abc_def0, 64), None);
    }

    #[test]
    fn mov_immediate() {
        let w = encode("mov", &["x0".into(), "#42".into()], &ctx()).unwrap();
        assert_eq!(w[0], 0xD280_0540);
        let w = encode("mov", &["w1".into(), "#0x1234".into()], &ctx()).unwrap();
        assert_eq!(w[0], 0x5282_4681);
    }

    #[test]
    fn add_sub() {
        let w = encode("add", &["x0".into(), "x1".into(), "#1".into()], &ctx()).unwrap();
        assert_eq!(w[0], 0x9100_0420);
        let w = encode("sub", &["sp".into(), "sp".into(), "#16".into()], &ctx()).unwrap();
        assert_eq!(w[0], 0xD100_43FF);
    }

    #[test]
    fn branches() {
        let c = Ctx { pc: 0, eval: &|s: &str| if s == "target" { Ok(8) } else { Err("x".into()) }, pool_offset: None, section_base: 0 };
        assert_eq!(encode("b", &["target".into()], &c).unwrap()[0], 0x1400_0002);
        assert_eq!(encode("b.eq", &["target".into()], &c).unwrap()[0], 0x5400_0040);
    }

    #[test]
    fn ret_enc() {
        assert_eq!(encode("ret", &[], &ctx()).unwrap()[0], 0xD65F_03C0);
    }
}
