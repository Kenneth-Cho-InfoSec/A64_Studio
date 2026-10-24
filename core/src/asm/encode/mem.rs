//! Load/store, pair, exclusive, prefetch and literal encoders.

use super::*;

#[derive(Debug, Clone, Copy)]
enum Mode {
    Offset,
    Pre,
    Post,
}

#[derive(Debug, Clone, Copy)]
enum Off {
    None,
    Imm(i64),
    Reg(Gpr, Option<(u32, i64)>),
}

#[derive(Debug, Clone, Copy)]
struct Mem {
    base: Gpr,
    off: Off,
    mode: Mode,
}

fn parse_mem(s: &str, op: usize) -> EResult<Mem> {
    let s = s.trim();
    if !s.starts_with('[') {
        return Err(EncError::at(format!("expected a memory operand like [x0, #8], found '{}'", s), op));
    }
    let close = s.find(']').ok_or_else(|| EncError::at("missing ']' in memory operand", op))?;
    let inside = &s[1..close];
    let after = s[close + 1..].trim();
    let parts = split_operands(inside);
    if parts.is_empty() {
        return Err(EncError::at("memory operand requires a base register", op));
    }
    let base = gpr(&parts[0], op)?;
    if base.width != 64 || !base.is_sp && is_zero_reg(base) {
        return Err(EncError::at("memory base must be an X register or SP", op));
    }
    let mut off = Off::None;
    if parts.len() >= 2 {
        let combined;
        let o = if parts.len() >= 3 {
            combined = format!("{} {}", parts[1].trim(), parts[2].trim());
            combined.as_str()
        } else {
            parts[1].trim()
        };
        if o.starts_with('#') || o.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) {
            let (core, suffix) = split_suffix(o);
            if suffix.is_some() {
                return Err(EncError::at("immediate memory offsets cannot have a shift", op));
            }
            let t = core.trim_start_matches('#');
            let v: i64 = t.parse().map_err(|_| EncError::at(format!("invalid offset '{}'", o), op))?;
            off = Off::Imm(v);
        } else {
            let (core, suffix) = split_suffix(o);
            let idx = gpr(core, op)?;
            if idx.is_sp || is_zero_reg(idx) {
                return Err(EncError::at("index register cannot be SP or XZR", op));
            }
            let ext = suffix.and_then(|(k, a)| {
                let k = k.to_ascii_lowercase();
                let opt = match k.as_str() {
                    "lsl" => 3u32,
                    "uxtw" => 2,
                    "sxtw" => 6,
                    "sxtx" => 7,
                    "uxtx" => 3,
                    _ => return None,
                };
                Some((opt, a.unwrap_or(0)))
            });
            if suffix.is_some() && ext.is_none() {
                return Err(EncError::at("invalid extend/shift for index register", op));
            }
            off = Off::Reg(idx, ext);
        }
    }
    let mode = if after == "!" {
        Mode::Pre
    } else if let Some(rest) = after.strip_prefix(',') {
        let rest = rest.trim();
        if !(rest.starts_with('#') || rest.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false)) {
            return Err(EncError::at("post-index offset must be an immediate", op));
        }
        let t = rest.trim_start_matches('#');
        let v: i64 = t.parse().map_err(|_| EncError::at(format!("invalid post-index offset '{}'", rest), op))?;
        if !matches!(off, Off::None | Off::Imm(0)) {
            return Err(EncError::at("cannot combine an offset with post-indexing", op));
        }
        off = Off::Imm(v);
        Mode::Post
    } else if after.is_empty() {
        Mode::Offset
    } else {
        return Err(EncError::at(format!("unexpected text after memory operand: '{}'", after), op));
    };
    Ok(Mem { base, off, mode })
}

/// Access description for a GPR/SIMD register: (size_log2, opc, v).
fn access_for(m: &str, ops: &[String], op: usize) -> EResult<(u32, u32, u32, u32)> {
    // returns (size, opc, v, scale_bytes)
    let t = ops[0].trim();
    let is_load = m.starts_with("ldr") || m.starts_with("ldur");
    match m {
        "ldr" | "str" | "ldur" | "stur" => {
            if let Some((vr, _)) = parse_vreg(t) {
                let (size, opc, scale) = match vr.class {
                    crate::syntax::VClass::B => (0, 1, 1),
                    crate::syntax::VClass::H => (1, 1, 2),
                    crate::syntax::VClass::S => (2, 1, 4),
                    crate::syntax::VClass::D => (3, 1, 8),
                    crate::syntax::VClass::Q => (0, if is_load { 3 } else { 2 }, 16),
                    _ => return Err(EncError::at("this vector register class cannot be loaded or stored directly", op)),
                };
                return Ok((size, opc, 1, scale));
            }
            let r = gpr(t, op)?;
            let size = if r.width == 64 { 3 } else { 2 };
            let scale = 1u32 << size;
            Ok((size, if is_load { 1 } else { 0 }, 0, scale))
        }
        "ldrb" | "strb" => {
            let r = gpr(t, op)?;
            if r.is_sp {
                return Err(EncError::at("expected a W register", op));
            }
            Ok((0, if is_load { 1 } else { 0 }, 0, 1))
        }
        "ldrh" | "strh" => {
            let r = gpr(t, op)?;
            if r.is_sp {
                return Err(EncError::at("expected a W register", op));
            }
            Ok((1, if is_load { 1 } else { 0 }, 0, 2))
        }
        "ldrsb" | "ldursb" => {
            let r = gpr(t, op)?;
            if r.width == 64 { Ok((0, 2, 0, 1)) } else { Ok((0, 3, 0, 1)) }
        }
        "ldrsh" | "ldursh" => {
            let r = gpr(t, op)?;
            if r.width == 64 { Ok((1, 2, 0, 2)) } else { Ok((1, 3, 0, 2)) }
        }
        "ldrsw" | "ldursw" => {
            let r = gpr(t, op)?;
            if r.width != 64 {
                return Err(EncError::at("ldrsw requires an X register destination", op));
            }
            Ok((2, 2, 0, 4))
        }
        "prfm" => Ok((3, 2, 0, 8)),
        _ => Err(EncError::new("internal: unhandled load/store")),
    }
}

fn encode_unsigned(size: u32, opc: u32, v: u32, scale: u32, imm: i64, rn: u32, rt: u32) -> EResult<u32> {
    if imm < 0 || imm % scale as i64 != 0 {
        return Err(EncError::new("offset is negative or not aligned; use the unscaled form (ldur/stur) or pre/post-indexing"));
    }
    let imm12 = imm / scale as i64;
    if imm12 > 4095 {
        return Err(EncError::new("offset out of range for the unsigned-offset form (max 4095 * access size)"));
    }
    let mut w = 0x3900_0000u32;
    w |= (size & 3) << 30;
    w |= (opc & 3) << 22;
    w |= (v & 1) << 26;
    w |= (imm12 as u32) << 10;
    w |= (rn & 31) << 5;
    w |= rt & 31;
    Ok(w)
}

fn encode_unscaled(size: u32, opc: u32, v: u32, imm: i64, mode: Mode, rn: u32, rt: u32) -> EResult<u32> {
    if !(-256..=255).contains(&imm) {
        return Err(EncError::new("offset out of range for the unscaled form (-256..255)"));
    }
    let mode_bits = match mode { Mode::Offset => 0b00, Mode::Post => 0b01, Mode::Pre => 0b11 };
    let mut w = 0x3800_0000u32;
    w |= (size & 3) << 30;
    w |= (opc & 3) << 22;
    w |= (v & 1) << 26;
    w |= ((imm as u32) & 0x1ff) << 12;
    w |= mode_bits << 10;
    w |= (rn & 31) << 5;
    w |= rt & 31;
    Ok(w)
}

#[allow(clippy::too_many_arguments)]
fn encode_reg_offset(size: u32, opc: u32, v: u32, rm: Gpr, ext: Option<(u32, i64)>, scale_log2: u32, rn: u32, rt: u32) -> EResult<u32> {
    let (option, shift) = match ext {
        Some((o, a)) => {
            if a != 0 && a != scale_log2 as i64 {
                return Err(EncError::new(format!("index shift must be 0 or {}", scale_log2)));
            }
            (o, if a != 0 { 1 } else { 0 })
        }
        None => (if rm.width == 64 { 3 } else { 2 }, 0),
    };
    if option == 3 || option == 7 {
        if rm.width != 64 {
            return Err(EncError::new("uxtx/sxtx require an X index register"));
        }
    } else if (option == 2 || option == 6) && rm.width != 32 {
        return Err(EncError::new("uxtw/sxtw require a W index register"));
    }
    let mut w = 0x3820_0800u32;
    w |= (size & 3) << 30;
    w |= (opc & 3) << 22;
    w |= (v & 1) << 26;
    w |= (rm.num as u32) << 16;
    w |= (option & 7) << 13;
    w |= shift << 12;
    w |= (rn & 31) << 5;
    w |= rt & 31;
    Ok(w)
}

pub fn encode(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    match m {
        "ldp" | "stp" => enc_pair(m, ops),
        "ldar" | "stlr" | "ldxr" | "stxr" | "ldaxr" | "stlxr" => enc_exclusive(m, ops),
        _ => enc_single(m, ops, ctx),
    }
}

fn enc_single(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    let is_load = m.starts_with("ldr") || m.starts_with("ldur") || m == "prfm";
    let want = if m == "stxr" || m == "stlxr" { 3 } else { 2 };
    if ops.len() != want {
        return Err(EncError::new(format!("'{}' takes {} operands", m, want)));
    }
    let (size, opc, v, scale) = access_for(m, ops, 0)?;
    let rt = if m == "prfm" {
        prfop(&ops[0], 0)?
    } else if let Some((vr, _)) = parse_vreg(&ops[0]) {
        vr.num as u32
    } else {
        let r = gpr(&ops[0], 0)?;
        if r.is_sp && m != "prfm" {
            return Err(EncError::at("SP cannot be loaded or stored directly", 0));
        }
        r.num as u32
    };

    // Literal form: ldr x0, label  (no brackets, not an '=' pool operand).
    let addr = ops[1].trim();
    if !addr.starts_with('[') {
        if !is_load {
            return Err(EncError::at("store instructions require a memory operand", 1));
        }
        if m == "ldrsw" {
            let target = (ctx.eval)(addr).map_err(|e| EncError::at(e, 1))?;
            let delta = target - ctx.pc as i64;
            if delta % 4 != 0 || !(-(1 << 20)..(1 << 20)).contains(&delta) {
                return Err(EncError::at("literal load target is out of range or unaligned", 1));
            }
            return Ok(vec![0x9800_0000 | (((delta / 4) as u32) & 0x7ffff) << 5 | (rt & 31)]);
        }
        if m == "prfm" {
            let target = (ctx.eval)(addr).map_err(|e| EncError::at(e, 1))?;
            let delta = target - ctx.pc as i64;
            if delta % 4 != 0 || !(-(1 << 20)..(1 << 20)).contains(&delta) {
                return Err(EncError::at("literal prefetch target is out of range", 1));
            }
            return Ok(vec![0xD800_0000 | (((delta / 4) as u32) & 0x7ffff) << 5 | (rt & 31)]);
        }
        let mut opc_lit = 0u32;
        let mut base = 0x1800_0000u32;
        if v == 1 {
            match parse_vreg(&ops[0]).map(|x| x.0.class) {
                Some(crate::syntax::VClass::S) => { base = 0x1C00_0000; }
                Some(crate::syntax::VClass::D) => { base = 0x5C00_0000; }
                Some(crate::syntax::VClass::Q) => { base = 0x9C00_0000; }
                _ => { opc_lit = 0; }
            }
        }
        let _ = opc_lit;
        if m == "ldr" && v == 1 && !matches!(parse_vreg(&ops[0]).map(|x| x.0.class), Some(crate::syntax::VClass::S | crate::syntax::VClass::D | crate::syntax::VClass::Q)) {
            // B/H literal loads do not exist.
            return Err(EncError::at("this vector size has no literal load form", 0));
        }
        let target = (ctx.eval)(addr).map_err(|e| EncError::at(e, 1))?;
        let delta = target - ctx.pc as i64;
        if delta % 4 != 0 || !(-(1 << 20)..(1 << 20)).contains(&delta) {
            return Err(EncError::at("literal load target is out of range or unaligned", 1));
        }
        return Ok(vec![base | (((delta / 4) as u32) & 0x7ffff) << 5 | (rt & 31)]);
    }

    let mem = parse_mem(addr, 1)?;
    let rn = mem.base.num as u32;
    match (mem.mode, mem.off) {
        (Mode::Offset, Off::Imm(imm)) => {
            if m.starts_with("ldur") || m.starts_with("stur") {
                encode_unscaled(size, opc, v, imm, Mode::Offset, rn, rt).map(|w| vec![w])
            } else if imm >= 0 && imm % scale as i64 == 0 {
                encode_unsigned(size, opc, v, scale, imm, rn, rt).map(|w| vec![w])
            } else {
                encode_unscaled(size, opc, v, imm, Mode::Offset, rn, rt).map(|w| vec![w])
            }
        }
        (Mode::Pre, Off::Imm(imm)) | (Mode::Post, Off::Imm(imm)) => {
            encode_unscaled(size, opc, v, imm, mem.mode, rn, rt).map(|w| vec![w])
        }
        (Mode::Offset, Off::Reg(rm, ext)) => {
            encode_reg_offset(size, opc, v, rm, ext, size, rn, rt).map(|w| vec![w])
        }
        (Mode::Offset, Off::None) => encode_unsigned(size, opc, v, scale, 0, rn, rt).map(|w| vec![w]),
        (_, Off::Reg(_, _)) => Err(EncError::at("pre/post-indexed forms require an immediate offset", 1)),
        (_, Off::None) => Err(EncError::at("this addressing mode requires an offset", 1)),
    }
}

fn prfop(s: &str, op: usize) -> EResult<u32> {
    let t = s.trim().to_ascii_lowercase();
    if let Ok(v) = t.parse::<u32>() {
        if v < 32 { return Ok(v); }
    }
    let (kind, level, strm) = if let Some(rest) = t.strip_prefix("pld") {
        (0u32, rest, false)
    } else if let Some(rest) = t.strip_prefix("pli") {
        (8u32, rest, false)
    } else if let Some(rest) = t.strip_prefix("pst") {
        (16u32, rest, false)
    } else {
        return Err(EncError::at(format!("unknown prefetch operation '{}'", s), op));
    };
    let (level, strm) = if let Some(l) = level.strip_suffix("keep") {
        (l, false)
    } else if let Some(l) = level.strip_suffix("strm") {
        (l, true)
    } else {
        (level, strm)
    };
    let lvl: u32 = match level {
        "l1" => 1,
        "l2" => 2,
        "l3" => 3,
        _ => return Err(EncError::at(format!("unknown prefetch level in '{}'", s), op)),
    };
    Ok(kind + (lvl - 1) * 2 + if strm { 1 } else { 0 })
}

fn enc_pair(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 3 {
        return Err(EncError::new(format!("'{}' takes two registers and a memory operand", m)));
    }
    let r1 = gpr(&ops[0], 0)?;
    let r2 = gpr(&ops[1], 1)?;
    if r1.width != r2.width || r1.is_sp || r2.is_sp {
        return Err(EncError::at("pair registers must be two W or two X registers", 1));
    }
    let mem = parse_mem(&ops[2], 2)?;
    let scale = if r1.width == 64 { 8 } else { 4 };
    let imm = match (mem.mode, mem.off) {
        (Mode::Offset, Off::Imm(v)) | (Mode::Pre, Off::Imm(v)) | (Mode::Post, Off::Imm(v)) => v,
        (_, Off::None) => 0,
        _ => return Err(EncError::at("pair addressing requires an immediate offset", 2)),
    };
    if imm % scale != 0 || !(-64 * scale..=63 * scale).contains(&imm) {
        return Err(EncError::at(format!("pair offset must be a multiple of {} in range {}..{}", scale, -64 * scale, 63 * scale), 2));
    }
    let imm7 = ((imm / scale) as u32) & 0x7f;
    let load = m == "ldp";
    let mode_bits = match mem.mode { Mode::Offset => 0b10u32, Mode::Post => 0b01, Mode::Pre => 0b11 };
    let mut w = 0x2800_0000u32;
    if r1.width == 64 { w |= 1 << 31; }
    w |= mode_bits << 23;
    if load { w |= 1 << 22; }
    w |= imm7 << 15;
    w |= (r2.num as u32) << 10;
    w |= (mem.base.num as u32) << 5;
    w |= r1.num as u32;
    Ok(vec![w])
}

fn enc_exclusive(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    let (want, has_status) = match m {
        "stxr" | "stlxr" => (3, true),
        _ => (2, false),
    };
    if ops.len() != want {
        return Err(EncError::new(format!("'{}' takes {} operands", m, want)));
    }
    let (status, value, addr) = if has_status {
        (Some(gpr(&ops[0], 0)?), gpr(&ops[1], 1)?, &ops[2])
    } else {
        (None, gpr(&ops[0], 0)?, &ops[1])
    };
    if value.width != 32 && value.width != 64 {
        return Err(EncError::at("expected a W or X register", 0));
    }
    if value.is_sp || value.is_zero() {
        return Err(EncError::at("exclusive access requires a real register", 0));
    }
    if let Some(st) = status {
        if st.width != 32 || st.is_sp || st.is_zero() {
            return Err(EncError::at("status register must be a W register", 0));
        }
    }
    let mem = parse_mem(addr, want - 1)?;
    if !matches!(mem.off, Off::None) || !matches!(mem.mode, Mode::Offset) {
        return Err(EncError::at("exclusive accesses use [Xn] with no offset", want - 1));
    }
    let sf = value.width == 64;
    let base = match m {
        "ldxr" => 0x085F_7C00u32,
        "ldaxr" => 0x085F_FC00,
        "stxr" => 0x0800_7C00,
        "stlxr" => 0x0800_FC00,
        "ldar" => 0x08DF_FC00,
        "stlr" => 0x089F_FC00,
        _ => return Err(EncError::new("unhandled exclusive access")),
    };
    let mut w = base;
    // 32-bit accesses set bit 31; 64-bit accesses set both size bits.
    w |= if sf { 0xC000_0000 } else { 0x8000_0000 };
    if let Some(st) = status {
        w |= (st.num as u32) << 16;
    }
    w |= (mem.base.num as u32) << 5;
    w |= value.num as u32;
    Ok(vec![w])
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
    fn loads() {
        assert_eq!(one("ldr", &["x0", "[x1]"]), 0xF940_0020);
        assert_eq!(one("ldr", &["x0", "[x1, #8]"]), 0xF940_0420);
        assert_eq!(one("str", &["x0", "[sp, #16]"]), 0xF900_0BE0);
        assert_eq!(one("ldrb", &["w0", "[x1]"]), 0x3940_0020);
        assert_eq!(one("ldrsw", &["x0", "[x1]"]), 0xB980_0020);
        assert_eq!(one("ldr", &["x0", "[x1, #-8]"]), 0xF85F_8020);
        assert_eq!(one("ldr", &["x0", "[x1, x2]"]), 0xF862_6820);
        assert_eq!(one("ldr", &["q0", "[x1]"]), 0x3DC0_0020);
    }
    #[test]
    fn pairs() {
        assert_eq!(one("stp", &["x29", "x30", "[sp, #-16]!"]), 0xA9BF_7BFD);
        assert_eq!(one("ldp", &["x29", "x30", "[sp], #16"]), 0xA8C1_7BFD);
        assert_eq!(one("stp", &["w0", "w1", "[x2, #8]"]), 0x2901_0440);
    }
    #[test]
    fn exclusives() {
        assert_eq!(one("ldar", &["x0", "[x1]"]), 0xC8DF_FC20);
        assert_eq!(one("stxr", &["w2", "x0", "[x1]"]), 0xC802_7C20);
    }
    #[test]
    fn prefetch() {
        assert_eq!(one("prfm", &["pldl1keep", "[x0]"]), 0xF980_0000);
    }
}
