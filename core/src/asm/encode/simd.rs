//! Advanced SIMD (NEON) and scalar floating-point encoders.
//!
//! This is an intentionally bounded subset: the operations needed for the
//! bundled examples and the common integer/FP vector cases. Anything outside
//! it returns a precise diagnostic instead of a wrong encoding.

use super::*;
use crate::syntax::{Arrangement, Elem, VClass, VReg};

pub fn handles(m: &str) -> bool {
    matches!(
        m,
        "add" | "sub" | "mul" | "and" | "orr" | "eor" | "bic" | "not" | "movi" | "mvni"
            | "dup" | "ins" | "umov" | "ld1" | "st1" | "ld1r" | "fadd" | "fsub" | "fmul"
            | "fdiv" | "fabs" | "fneg" | "fsqrt" | "fcvt" | "scvtf" | "ucvtf" | "fcvtzs"
            | "fcvtzu" | "fcmp" | "fcmpe" | "fmov" | "addv" | "addp" | "aese" | "aesd"
            | "aesmc" | "aesimc" | "sha256h" | "sha256h2" | "sha256su0" | "sha256su1"
            | "pmull" | "pmull2"
    )
}

fn arr_of(v: VReg, op: usize) -> EResult<Arrangement> {
    v.arr.ok_or_else(|| EncError::at("a vector arrangement such as .16b or .4s is required", op))
}

fn size_of(arr: Arrangement, op: usize) -> EResult<u32> {
    let total = arr.count as u16 * arr.elem.size() as u16;
    if total != 64 && total != 128 {
        return Err(EncError::at("vector arrangement must describe 64 or 128 bits", op));
    }
    Ok(match arr.elem {
        Elem::B => 0,
        Elem::H => 1,
        Elem::S => 2,
        Elem::D => 3,
        Elem::Q => return Err(EncError::at("invalid element size", op)),
    })
}

fn is_q(arr: Arrangement) -> bool {
    arr.count as u16 * arr.elem.size() as u16 == 128
}

fn vec3(ops: &[String]) -> EResult<(VReg, VReg, VReg, Arrangement)> {
    if ops.len() != 3 {
        return Err(EncError::new("this vector instruction takes three operands"));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let (vn, _) = vreg(&ops[1], 1)?;
    let (vm, _) = vreg(&ops[2], 2)?;
    if vd.class != VClass::V || vn.class != VClass::V || vm.class != VClass::V {
        return Err(EncError::at("expected V registers (not Q/D/S/H/B) for this vector instruction", 0));
    }
    let a = arr_of(vd, 0)?;
    if arr_of(vn, 1)? != a || arr_of(vm, 2)? != a {
        return Err(EncError::at("all vector operands must have the same arrangement", 2));
    }
    Ok((vd, vn, vm, a))
}

fn vec2(ops: &[String]) -> EResult<(VReg, VReg, Arrangement)> {
    if ops.len() != 2 {
        return Err(EncError::new("this vector instruction takes two operands"));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let (vn, _) = vreg(&ops[1], 1)?;
    let a = arr_of(vd, 0)?;
    if arr_of(vn, 1)? != a {
        return Err(EncError::at("both vector operands must have the same arrangement", 1));
    }
    Ok((vd, vn, a))
}

fn scalar_class(v: VReg) -> Option<(u32, u32)> {
    // (type bit, width)
    match v.class {
        VClass::S => Some((0, 32)),
        VClass::D => Some((1, 64)),
        _ => None,
    }
}

fn lane_imm5(arr: Arrangement, idx: u8, op: usize) -> EResult<u32> {
    let max = if arr.count == 0 { (128 / arr.elem.size()) - 1 } else { arr.count - 1 };
    if idx > max {
        return Err(EncError::at(format!("lane index must be 0..{}", max), op));
    }
    let (shift, base) = match arr.elem {
        Elem::B => (1, 1),
        Elem::H => (2, 2),
        Elem::S => (3, 4),
        Elem::D => (4, 8),
        Elem::Q => return Err(EncError::at("invalid lane element", op)),
    };
    Ok(((idx as u32) << shift) | base)
}

pub fn encode(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    match m {
        "add" | "sub" | "mul" | "and" | "orr" | "eor" | "bic" => enc_vec_arith(m, ops),
        "not" => {
            let (vd, vn, a) = vec2(ops)?;
            if a.elem != Elem::B {
                return Err(EncError::at("'not' operates on .8b or .16b vectors", 1));
            }
            Ok(vec![0x2E20_5800 | (is_q(a) as u32) << 30 | (vn.num as u32) << 5 | vd.num as u32])
        }
        "addp" => {
            let (vd, vn, vm, a) = vec3(ops)?;
            let size = size_of(a, 0)?;
            Ok(vec![0x0E20_BC00 | (is_q(a) as u32) << 30 | size << 22 | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32])
        }
        "addv" => {
            if ops.len() != 2 {
                return Err(EncError::new("'addv' takes a scalar destination and a vector source"));
            }
            let (vd, _) = vreg(&ops[0], 0)?;
            let (vn, _) = vreg(&ops[1], 1)?;
            let a = arr_of(vn, 1)?;
            let size = size_of(a, 1)?;
            if size == 3 && !is_q(a) {
                return Err(EncError::at("2d addv is not encodable", 1));
            }
            Ok(vec![0x0E31_B800 | (is_q(a) as u32) << 30 | size << 22 | (vn.num as u32) << 5 | vd.num as u32])
        }
        "movi" | "mvni" => enc_movi(m, ops, ctx),
        "dup" => enc_dup(ops),
        "ins" => enc_ins(ops),
        "umov" => enc_umov(ops),
        "ld1" | "st1" => enc_ld1(m, ops),
        "ld1r" => enc_ld1r(ops),
        "fadd" | "fsub" | "fmul" | "fdiv" | "fabs" | "fneg" | "fsqrt" => enc_fp(m, ops),
        "fcvt" | "scvtf" | "ucvtf" | "fcvtzs" | "fcvtzu" => enc_fp_convert(m, ops),
        "fcmp" | "fcmpe" => enc_fcmp(m, ops),
        "fmov" => enc_fmov(ops),
        "aese" | "aesd" | "aesmc" | "aesimc" => {
            let (vd, vn, a) = vec2(ops)?;
            if a.elem != Elem::B || !is_q(a) {
                return Err(EncError::at("AES instructions require .16b vectors", 1));
            }
            let base = match m { "aese" => 0x4E28_4800u32, "aesd" => 0x4E28_5800, "aesmc" => 0x4E28_6800, _ => 0x4E28_7800 };
            Ok(vec![base | (vn.num as u32) << 5 | vd.num as u32])
        }
        "sha256h" | "sha256h2" | "sha256su0" | "sha256su1" => enc_sha(m, ops),
        "pmull" | "pmull2" => {
            if ops.len() != 3 {
                return Err(EncError::new("pmull takes three vector operands"));
            }
            let (vd, _) = vreg(&ops[0], 0)?;
            let (vn, _) = vreg(&ops[1], 1)?;
            let (vm, _) = vreg(&ops[2], 2)?;
            let da = arr_of(vd, 0)?;
            let na = arr_of(vn, 1)?;
            let ma = arr_of(vm, 2)?;
            if da.elem != Elem::H || da.count != 8 || na.elem != Elem::B || ma.elem != Elem::B {
                return Err(EncError::at("pmull requires Vd.8h, Vn.8b, Vm.8b", 0));
            }
            let q = (m == "pmull2") as u32;
            Ok(vec![0x0E20_E000 | q << 30 | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32])
        }
        _ => Err(EncError::new(format!("unhandled SIMD instruction '{}'", m))),
    }
}

fn enc_vec_arith(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    let (vd, vn, vm, a) = vec3(ops)?;
    let size = size_of(a, 0)?;
    if matches!(m, "and" | "orr" | "eor" | "bic") {
        if a.elem != Elem::B {
            return Err(EncError::at("bitwise vector instructions require .8b or .16b", 0));
        }
        let base = match m {
            "and" => 0x0E20_1C00u32,
            "orr" => 0x0EA0_1C00,
            "eor" => 0x2E20_1C00,
            _ => 0x0E60_1C00,
        };
        return Ok(vec![base | (is_q(a) as u32) << 30 | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32]);
    }
    let base = match m {
        "add" => 0x0E20_8400u32,
        "sub" => 0x2E20_8400,
        "mul" => 0x0E20_9C00,
        _ => return Err(EncError::new("unhandled vector arithmetic")),
    };
    if m == "mul" && size == 3 {
        return Err(EncError::at("integer vector multiply does not support 2d", 0));
    }
    Ok(vec![base | (is_q(a) as u32) << 30 | size << 22 | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32])
}

fn enc_movi(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new(format!("'{}' takes a vector register and an immediate", m)));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let a = arr_of(vd, 0)?;
    let (core, suffix) = split_suffix(&ops[1]);
    let v = ctx.imm(core, 1)?;
    let shift = match suffix {
        None => 0,
        Some((k, amt)) if k.eq_ignore_ascii_case("lsl") => amt.unwrap_or(0),
        Some((k, _)) => return Err(EncError::at(format!("only lsl is allowed, found '{}'", k), 1)),
    };
    if !(0..=0xff).contains(&v) {
        return Err(EncError::at("the immediate must be an 8-bit value (0..255)", 1));
    }
    let imm8 = v as u32;
    let (cmode, op) = match a.elem {
        Elem::B => {
            if shift != 0 { return Err(EncError::at("byte immediates do not take a shift", 1)); }
            (0xE, 0)
        }
        Elem::H => match shift {
            0 => (0x8, 0),
            8 => (0xA, 0),
            _ => return Err(EncError::at("halfword shift must be 0 or 8", 1)),
        },
        Elem::S => match shift {
            0 => (0x0, 0),
            8 => (0x2, 0),
            16 => (0x4, 0),
            24 => (0x6, 0),
            _ => return Err(EncError::at("word shift must be 0, 8, 16 or 24", 1)),
        },
        Elem::D => {
            if a.count != 2 || shift != 0 {
                return Err(EncError::at("doubleword immediates require .2d and no shift", 1));
            }
            if imm8 != 0 {
                return Err(EncError::at(
                    "2d immediates other than #0 are not implemented by this assembler; use dup or a literal pool",
                    1,
                ));
            }
            (0xE, 1)
        }
        Elem::Q => return Err(EncError::at("invalid arrangement", 0)),
    };
    let op_bit = if m == "mvni" { op ^ 1 } else { op };
    if m == "mvni" && (a.elem == Elem::B || a.elem == Elem::D) {
        return Err(EncError::at("mvni requires .4h/.8h or .2s/.4s", 0));
    }
    // The imm8 field is split: bits 7:5 at [18:16], bits 4:0 at [9:5].
    let imm_field = ((imm8 & 0xE0) << 11) | ((imm8 & 0x1F) << 5);
    let w = 0x0F00_0400u32
        | (is_q(a) as u32) << 30
        | (op_bit as u32) << 29
        | (cmode << 12)
        | imm_field
        | vd.num as u32;
    Ok(vec![w])
}

fn enc_dup(ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new("'dup' takes a vector register and a source"));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let a = arr_of(vd, 0)?;
    let src = ops[1].trim();
    if let Some(r) = parse_gpr(src) {
        let imm5 = match a.elem {
            Elem::B => 1,
            Elem::H => 2,
            Elem::S => 4,
            Elem::D => 8,
            Elem::Q => return Err(EncError::at("invalid element size", 0)),
        };
        if a.elem == Elem::D && r.width != 64 {
            return Err(EncError::at("2d dup requires an X register", 1));
        }
        if a.elem != Elem::D && r.width != 32 {
            return Err(EncError::at("dup requires a W register for byte/halfword/word elements", 1));
        }
        return Ok(vec![0x0E00_0C00 | (is_q(a) as u32) << 30 | imm5 << 16 | (r.num as u32) << 5 | vd.num as u32]);
    }
    let (vn, idx) = vreg(src, 1)?;
    let idx = idx.ok_or_else(|| EncError::at("expected a lane index such as v1.s[0]", 1))?;
    let na = arr_of(vn, 1)?;
    if na.elem != a.elem {
        return Err(EncError::at("source and destination element sizes must match", 1));
    }
    let imm5 = lane_imm5(na, idx, 1)?;
    let imm4 = if na.count > 1 { (idx as u32) >> (match na.elem { Elem::B => 3, Elem::H => 2, Elem::S => 1, Elem::D => 0, _ => 0 }) } else { 0 };
    Ok(vec![0x0E00_0400 | (is_q(a) as u32) << 30 | imm5 << 16 | imm4 << 11 | (vn.num as u32) << 5 | vd.num as u32])
}

fn enc_ins(ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new("'ins' takes a vector lane and a W register"));
    }
    let (vd, idx) = vreg(&ops[0], 0)?;
    let idx = idx.ok_or_else(|| EncError::at("expected a destination lane such as v0.s[0]", 0))?;
    let a = arr_of(vd, 0)?;
    let imm5 = lane_imm5(a, idx, 0)?;
    let r = gpr(&ops[1], 1)?;
    if r.width != 32 {
        return Err(EncError::at("ins requires a W register", 1));
    }
    Ok(vec![0x4E00_1C00 | imm5 << 16 | (r.num as u32) << 5 | vd.num as u32])
}

fn enc_umov(ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new("'umov' takes a W register and a vector lane"));
    }
    let r = gpr(&ops[0], 0)?;
    if r.width != 32 {
        return Err(EncError::at("umov requires a W destination", 0));
    }
    let (vn, idx) = vreg(&ops[1], 1)?;
    let idx = idx.ok_or_else(|| EncError::at("expected a lane index", 1))?;
    let a = arr_of(vn, 1)?;
    let imm5 = lane_imm5(a, idx, 1)?;
    Ok(vec![0x0E00_3C00 | imm5 << 16 | (vn.num as u32) << 5 | r.num as u32])
}

fn enc_ld1(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() < 2 {
        return Err(EncError::new(format!("'{}' takes a register list and a memory operand", m)));
    }
    let list = ops[0].trim();
    let inner = list.strip_prefix('{').and_then(|s| s.strip_suffix('}')).ok_or_else(|| EncError::at("expected a register list like {v0.16b}", 0))?;
    let regs = split_operands(inner);
    if regs.len() != 1 {
        return Err(EncError::at("this assembler currently supports single-register ld1/st1 lists; multi-register lists are not implemented", 0));
    }
    let (vt, _) = vreg(&regs[0], 0)?;
    let a = arr_of(vt, 0)?;
    let size_code = match a.elem { Elem::B => 0, Elem::H => 1, Elem::S => 2, Elem::D => 3, Elem::Q => return Err(EncError::at("invalid arrangement", 0)) };
    let addr = ops[1].trim();
    if !addr.starts_with('[') {
        return Err(EncError::at("expected a memory operand", 1));
    }
    let mem = parse_ldst_addr(addr, 1)?;
    if mem.post.is_some() {
        return Err(EncError::at("post-indexed ld1/st1 is not implemented", 1));
    }
    let load = m == "ld1";
    let mut w = 0x0C00_7000u32;
    if load { w |= 1 << 22; }
    w |= (is_q(a) as u32) << 30;
    w |= size_code << 10;
    w |= (mem.base.num as u32) << 5;
    w |= vt.num as u32;
    Ok(vec![w])
}

fn enc_ld1r(ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new("'ld1r' takes a register and a memory operand"));
    }
    let list = ops[0].trim();
    let inner = list.strip_prefix('{').and_then(|s| s.strip_suffix('}')).ok_or_else(|| EncError::at("expected a register list like {v0.4s}", 0))?;
    let regs = split_operands(inner);
    if regs.len() != 1 {
        return Err(EncError::at("ld1r supports a single register in this build", 0));
    }
    let (vt, _) = vreg(&regs[0], 0)?;
    let a = arr_of(vt, 0)?;
    let size = size_of(a, 0)?;
    let addr = ops[1].trim();
    if !addr.starts_with('[') {
        return Err(EncError::at("expected a memory operand", 1));
    }
    let mem = parse_ldst_addr(addr, 1)?;
    if mem.post.is_some() {
        return Err(EncError::at("post-indexed ld1r is not implemented", 1));
    }
    Ok(vec![0x0D40_C000 | (is_q(a) as u32) << 30 | size << 10 | (mem.base.num as u32) << 5 | vt.num as u32])
}

struct LdAddr {
    base: Gpr,
    post: Option<i64>,
}

fn parse_ldst_addr(s: &str, op: usize) -> EResult<LdAddr> {
    let close = s.find(']').ok_or_else(|| EncError::at("missing ']'", op))?;
    let inside = &s[1..close];
    let parts = split_operands(inside);
    if parts.len() != 1 {
        return Err(EncError::at("this form requires [Xn] without an offset", op));
    }
    let base = gpr(&parts[0], op)?;
    if base.width != 64 {
        return Err(EncError::at("base must be an X register or SP", op));
    }
    let after = s[close + 1..].trim();
    let post = if after.is_empty() {
        None
    } else if let Some(rest) = after.strip_prefix(',') {
        let t = rest.trim().trim_start_matches('#');
        Some(t.parse::<i64>().map_err(|_| EncError::at("invalid post-index offset", op))?)
    } else {
        return Err(EncError::at("unexpected text after memory operand", op));
    };
    Ok(LdAddr { base, post })
}

fn enc_fp(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    // Scalar form: S/D registers.
    if let Some((vd0, _)) = parse_vreg(&ops[0]) {
        if scalar_class(vd0).is_some() {
            return enc_fp_scalar(m, ops);
        }
    }
    let arity = if matches!(m, "fabs" | "fneg" | "fsqrt") { 2 } else { 3 };
    if ops.len() != arity {
        return Err(EncError::new(format!("'{}' takes {} operands", m, arity)));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let (vn, _) = vreg(&ops[1], 1)?;
    let a = arr_of(vd, 0)?;
    if arr_of(vn, 1)? != a {
        return Err(EncError::at("vector arrangements must match", 1));
    }
    let size = size_of(a, 0)?;
    if !((size == 2 && is_q(a)) || (size == 3)) {
        return Err(EncError::at("floating-point vectors must be .4s or .2d", 0));
    }
    let fp_sz = if a.elem == Elem::D { 1u32 } else { 0u32 };
    let mut w = match m {
        "fadd" => 0x0E20_D400u32,
        "fsub" => 0x0EA0_D400,
        "fmul" => 0x2E20_DC00,
        "fdiv" => 0x2E20_FC00,
        "fabs" => 0x0EA0_F800,
        "fneg" => 0x2EA0_F800,
        _ => 0x2EA1_F800,
    };
    w |= (is_q(a) as u32) << 30 | fp_sz << 22;
    if arity == 3 {
        let (vm, _) = vreg(&ops[2], 2)?;
        if arr_of(vm, 2)? != a {
            return Err(EncError::at("vector arrangements must match", 2));
        }
        w |= (vm.num as u32) << 16;
    }
    w |= (vn.num as u32) << 5 | vd.num as u32;
    Ok(vec![w])
}

fn enc_fp_scalar(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    let arity = if matches!(m, "fabs" | "fneg" | "fsqrt") { 2 } else { 3 };
    if ops.len() != arity {
        return Err(EncError::new(format!("'{}' takes {} operands", m, arity)));
    }
    let (vd, _) = vreg(&ops[0], 0)?;
    let (vn, _) = vreg(&ops[1], 1)?;
    let (ty, _) = scalar_class(vd).ok_or_else(|| EncError::at("expected an S or D register", 0))?;
    let (ty2, _) = scalar_class(vn).ok_or_else(|| EncError::at("expected an S or D register", 1))?;
    if ty != ty2 {
        return Err(EncError::at("scalar FP operands must have the same precision", 1));
    }
    let base = match m {
        "fadd" => 0x1E20_2800u32,
        "fsub" => 0x1E20_3800,
        "fmul" => 0x1E20_0800,
        "fdiv" => 0x1E20_1800,
        "fabs" => 0x1E20_C000,
        "fneg" => 0x1E21_4000,
        _ => 0x1E21_C000,
    };
    let mut w = base | ty << 22;
    if arity == 3 {
        let (vm, _) = vreg(&ops[2], 2)?;
        let (ty3, _) = scalar_class(vm).ok_or_else(|| EncError::at("expected an S or D register", 2))?;
        if ty3 != ty {
            return Err(EncError::at("scalar FP operands must have the same precision", 2));
        }
        w |= (vm.num as u32) << 16;
    }
    w |= (vn.num as u32) << 5 | vd.num as u32;
    Ok(vec![w])
}

fn enc_fp_convert(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new(format!("'{}' takes two operands", m)));
    }
    if m == "fcvt" {
        let (vd, _) = vreg(&ops[0], 0)?;
        let (vn, _) = vreg(&ops[1], 1)?;
        let (dt, _) = scalar_class(vd).ok_or_else(|| EncError::at("expected S or D destination", 0))?;
        let (nt, _) = scalar_class(vn).ok_or_else(|| EncError::at("expected S or D source", 1))?;
        if dt == nt {
            return Err(EncError::at("fcvt requires different source and destination precision", 0));
        }
        let w = if nt == 1 {
            0x1E62_4000u32
        } else {
            0x1E22_C000u32
        } | (vn.num as u32) << 5 | vd.num as u32;
        return Ok(vec![w]);
    }
    let first = parse_vreg(&ops[0]).map(|x| x.0);
    if let Some(fv) = first.filter(|v| scalar_class(*v).is_some()) {
        // scvtf/ucvtf Sd, Wn/Xn
        let r = gpr(&ops[1], 1)?;
        let (ty, _) = scalar_class(fv).unwrap();
        let sf = r.width == 64;
        let base = if m == "scvtf" { 0x1E22_0000u32 } else { 0x1E23_0000 };
        return Ok(vec![base | (sf as u32) << 31 | ty << 22 | (r.num as u32) << 5 | fv.num as u32]);
    }
    // fcvtzs/fcvtzu Wd/Xd, Sn/Dn
    let r = gpr(&ops[0], 0)?;
    let (fv, _) = vreg(&ops[1], 1)?;
    let (ty, _) = scalar_class(fv).ok_or_else(|| EncError::at("expected S or D source", 1))?;
    let base = if m == "fcvtzs" { 0x1E38_0000u32 } else { 0x1E39_0000 };
    Ok(vec![base | (((r.width == 64) as u32) * (1 << 31)) | ty << 22 | (fv.num as u32) << 5 | r.num as u32])
}

fn enc_fcmp(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new(format!("'{}' takes two operands", m)));
    }
    let (vn, _) = vreg(&ops[0], 0)?;
    let (ty, _) = scalar_class(vn).ok_or_else(|| EncError::at("expected an S or D register", 0))?;
    let mut w = 0x1E20_2000u32 | ty << 22;
    if m == "fcmpe" { w |= 0x10; }
    let second = ops[1].trim();
    if let Some((vm, _)) = parse_vreg(second) {
        let (ty2, _) = scalar_class(vm).ok_or_else(|| EncError::at("expected an S or D register", 1))?;
        if ty2 != ty {
            return Err(EncError::at("precision mismatch", 1));
        }
        w |= (vm.num as u32) << 16;
    } else {
        // fcmp s0, #0.0
        let t = second.trim_start_matches('#');
        let v: f64 = t.parse().map_err(|_| EncError::at("expected a register or #0.0", 1))?;
        if v != 0.0 {
            return Err(EncError::at("only #0.0 is encodable in the immediate form", 1));
        }
        w |= 0x8;
    }
    w |= (vn.num as u32) << 5;
    Ok(vec![w])
}

fn enc_fmov(ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new("'fmov' takes two operands"));
    }
    let dst = parse_vreg(&ops[0]);
    let src = parse_vreg(&ops[1]);
    if let (Some((vd, _)), Some((vn, _))) = (dst, src) {
        let (dt, _) = scalar_class(vd).ok_or_else(|| EncError::at("expected S or D registers", 0))?;
        let (st, _) = scalar_class(vn).ok_or_else(|| EncError::at("expected S or D registers", 1))?;
        if dt != st {
            return Err(EncError::at("fmov requires matching precision", 1));
        }
        return Ok(vec![0x1E20_4000 | dt << 22 | (vn.num as u32) << 5 | vd.num as u32]);
    }
    if let Some((vd, _)) = dst {
        let (ty, _) = scalar_class(vd).ok_or_else(|| EncError::at("expected S or D destination", 0))?;
        let r = gpr(&ops[1], 1)?;
        let base = if ty == 0 {
            if r.width != 32 { return Err(EncError::at("fmov Sd, Wn requires a W register", 1)); }
            0x1E27_0000u32
        } else {
            if r.width != 64 { return Err(EncError::at("fmov Dd, Xn requires an X register", 1)); }
            0x9E67_0000
        };
        return Ok(vec![base | (r.num as u32) << 5 | vd.num as u32]);
    }
    let r = gpr(&ops[0], 0)?;
    let (vn, _) = src.ok_or_else(|| EncError::at("expected a scalar FP register", 1))?;
    let (ty, _) = scalar_class(vn).ok_or_else(|| EncError::at("expected S or D source", 1))?;
    let base = if ty == 0 {
        if r.width != 32 { return Err(EncError::at("fmov Wd, Sn requires a W register", 0)); }
        0x1E26_0000u32
    } else {
        if r.width != 64 { return Err(EncError::at("fmov Xd, Dn requires an X register", 0)); }
        0x9E66_0000
    };
    Ok(vec![base | (vn.num as u32) << 5 | r.num as u32])
}

fn enc_sha(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    match m {
        "sha256h" | "sha256h2" => {
            if ops.len() != 3 {
                return Err(EncError::new("sha256h takes Qd, Qn, Vm.4s"));
            }
            let (vd, _) = vreg(&ops[0], 0)?;
            let (vn, _) = vreg(&ops[1], 1)?;
            let (vm, _) = vreg(&ops[2], 2)?;
            if vd.class != VClass::Q || vn.class != VClass::Q {
                return Err(EncError::at("sha256h requires Q registers for the first two operands", 0));
            }
            if arr_of(vm, 2)?.elem != Elem::S {
                return Err(EncError::at("third operand must be .4s", 2));
            }
            let base = if m == "sha256h" { 0x5E00_4000u32 } else { 0x5E00_5000 };
            Ok(vec![base | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32])
        }
        "sha256su0" => {
            let (vd, vn, a) = vec2(ops)?;
            if a.elem != Elem::S || !is_q(a) {
                return Err(EncError::at("sha256su0 requires .4s vectors", 0));
            }
            Ok(vec![0x5E28_2800 | (vn.num as u32) << 5 | vd.num as u32])
        }
        _ => {
            if ops.len() != 3 {
                return Err(EncError::new("sha256su1 takes three vectors"));
            }
            let (vd, _) = vreg(&ops[0], 0)?;
            let (vn, _) = vreg(&ops[1], 1)?;
            let (vm, _) = vreg(&ops[2], 2)?;
            if arr_of(vd, 0)?.elem != Elem::S || arr_of(vn, 1)?.elem != Elem::S || arr_of(vm, 2)?.elem != Elem::S {
                return Err(EncError::at("sha256su1 requires .4s vectors", 0));
            }
            Ok(vec![0x5E00_6000 | (vm.num as u32) << 16 | (vn.num as u32) << 5 | vd.num as u32])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn no_eval(_: &str) -> Result<i64, String> { Err("no symbols".into()) }
    fn c() -> Ctx<'static> { Ctx { pc: 0, eval: &no_eval, pool_offset: None, section_base: 0 } }
    fn one(m: &str, ops: &[&str]) -> u32 {
        let v: Vec<String> = ops.iter().map(|s| s.to_string()).collect();
        encode(m, &v, &c()).unwrap()[0]
    }
    #[test]
    fn vector_int() {
        assert_eq!(one("add", &["v0.16b", "v1.16b", "v2.16b"]), 0x4E22_8420);
        assert_eq!(one("add", &["v0.4s", "v1.4s", "v2.4s"]), 0x4EA2_8420);
        assert_eq!(one("mul", &["v0.4s", "v1.4s", "v2.4s"]), 0x4EA2_9C20);
        assert_eq!(one("and", &["v0.16b", "v1.16b", "v2.16b"]), 0x4E22_1C20);
    }
    #[test]
    fn vector_fp() {
        assert_eq!(one("fadd", &["v0.4s", "v1.4s", "v2.4s"]), 0x4E22_D420);
        assert_eq!(one("fmul", &["v0.4s", "v1.4s", "v2.4s"]), 0x6E22_DC20);
    }
    #[test]
    fn scalar_fp() {
        assert_eq!(one("fadd", &["s0", "s1", "s2"]), 0x1E22_2820);
        assert_eq!(one("fadd", &["d0", "d1", "d2"]), 0x1E62_2820);
        assert_eq!(one("fcmp", &["s0", "s1"]), 0x1E21_2000);
    }
    #[test]
    fn moves() {
        assert_eq!(one("movi", &["v0.16b", "#0"]), 0x4F00_E400);
        assert_eq!(one("movi", &["v0.4s", "#1"]), 0x4F00_0420);
        assert_eq!(one("dup", &["v0.4s", "w1"]), 0x4E04_0C20);
    }
    #[test]
    fn ldst() {
        assert_eq!(one("ld1", &["{v0.16b}", "[x1]"]), 0x4C40_7020);
        assert_eq!(one("st1", &["{v0.16b}", "[x1]"]), 0x4C00_7020);
    }
}
