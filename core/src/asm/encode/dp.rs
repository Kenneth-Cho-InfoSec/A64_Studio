//! Data-processing, move, bitfield, branch-adjacent and arithmetic encoders.

use super::*;

fn zero_for(first: Option<&String>) -> String {
    match first.and_then(|s| parse_gpr(s)) {
        Some(g) if g.width == 32 => "wzr".to_string(),
        _ => "xzr".to_string(),
    }
}

fn expect(ops: &[String], n: usize, what: &str) -> EResult<()> {
    if ops.len() != n {
        return Err(EncError::new(format!("{} takes {} operand{}", what, n, if n == 1 { "" } else { "s" })));
    }
    Ok(())
}

fn imm_shift(s: &str, ctx: &Ctx, op: usize) -> EResult<(i64, Option<i64>)> {
    let (core, suffix) = split_suffix(s);
    let v = ctx.imm(core, op)?;
    let sh = match suffix {
        None => None,
        Some((kind, amt)) if kind.eq_ignore_ascii_case("lsl") => Some(amt.unwrap_or(0)),
        Some((kind, _)) => {
            return Err(EncError::at(format!("only 'lsl' is allowed here, found '{}'", kind), op))
        }
    };
    Ok((v, sh))
}

pub fn encode(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    match m {
        "mov" => enc_mov(ops, ctx),
        "movz" | "movn" | "movk" => enc_movwide(m, ops, ctx),
        "add" | "adds" | "sub" | "subs" => enc_addsub(m, ops, ctx),
        "cmp" | "cmn" => {
            let (sub, setflags) = if m == "cmp" { (true, true) } else { (false, true) };
            let zr = zero_for(ops.first());
            let mut v = vec![zr];
            v.extend_from_slice(ops);
            enc_addsub_impl(sub, setflags, &v, ctx, true)
        }
        "neg" | "negs" => {
            let sub = true;
            let setflags = m == "negs";
            if ops.len() < 2 || ops.len() > 3 {
                return Err(EncError::new("'neg' takes a destination, a source and an optional shift"));
            }
            let mut v = vec![ops[0].clone(), zero_for(ops.first()), ops[1].clone()];
            if ops.len() == 3 {
                v.push(ops[2].clone());
            }
            enc_addsub_impl(sub, setflags, &v, ctx, false)
        }
        "and" | "orr" | "eor" | "ands" | "bic" | "orn" | "eon" | "bics" => enc_logical(m, ops, ctx),
        "tst" => {
            if ops.len() < 2 || ops.len() > 3 {
                return Err(EncError::new("'tst' takes two operands and an optional shift"));
            }
            let mut v = vec![zero_for(ops.first())];
            v.extend_from_slice(ops);
            enc_logical("ands", &v, ctx)
        }
        "mvn" => {
            if ops.len() < 2 || ops.len() > 3 {
                return Err(EncError::new("'mvn' takes a destination, a source and an optional shift"));
            }
            let mut v = vec![ops[0].clone(), zero_for(ops.first()), ops[1].clone()];
            if ops.len() == 3 {
                v.push(ops[2].clone());
            }
            enc_logical("orn", &v, ctx)
        }
        "lsl" | "lsr" | "asr" | "ror" => enc_shift_alias(m, ops, ctx),
        "lslv" | "lsrv" | "asrv" | "rorv" => {
            expect(ops, 3, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            check_same_width(rd, rn, 0)?;
            check_same_width(rn, rm, 2)?;
            if rd.is_sp || rn.is_sp || rm.is_sp || is_zero_reg(rd) || is_zero_reg(rn) || is_zero_reg(rm) {
                return Err(EncError::new("shift instructions do not accept SP or the zero register as operands"));
            }
            let opcode = match m {
                "lslv" => 8,
                "lsrv" => 9,
                "asrv" => 10,
                _ => 11,
            };
            Ok(vec![insn_dp2(rd.width == 64, opcode, rm.num as u32, rn.num as u32, rd.num as u32)])
        }
        "ubfm" | "sbfm" | "bfm" => {
            expect(ops, 4, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            check_same_width(rd, rn, 1)?;
            if rd.is_sp || rn.is_sp {
                return Err(EncError::new("bitfield instructions do not accept SP"));
            }
            let bits = rd.width as u32;
            let immr = ctx.imm_u(&ops[2], 2, 6)? as u32;
            let imms = ctx.imm_u(&ops[3], 3, 6)? as u32;
            if immr >= bits || imms >= bits {
                return Err(EncError::at(format!("bitfield position must be below {}", bits), 2));
            }
            let opc = match m { "sbfm" => 0, "bfm" => 1, _ => 2 };
            Ok(vec![insn_bitfield(rd.width == 64, opc, immr, imms, rn.num as u32, rd.num as u32)])
        }
        "ubfx" | "sbfx" | "bfxil" | "bfi" | "sbfiz" | "ubfiz" => enc_bitfield_alias(m, ops, ctx),
        "sxtb" | "sxth" | "sxtw" | "uxtb" | "uxth" | "uxtw" => enc_extend(m, ops),
        "rev" | "rev16" | "rev32" | "rbit" | "clz" | "cls" => {
            expect(ops, 2, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            check_same_width(rd, rn, 1)?;
            if rd.is_sp || rn.is_sp {
                return Err(EncError::new("this instruction does not accept SP"));
            }
            if m == "rev32" && rd.width != 64 {
                return Err(EncError::at("rev32 requires 64-bit registers", 0));
            }
            // 32-bit REV uses opcode 2; the 64-bit form uses 3.
            let opc = match m {
                "rbit" => 0,
                "rev16" => 1,
                "rev32" => 2,
                "rev" => if rd.width == 64 { 3 } else { 2 },
                "clz" => 4,
                _ => 5,
            };
            Ok(vec![insn_dp1(rd.width == 64, opc, rn.num as u32, rd.num as u32)])
        }
        "csel" | "csinc" | "csinv" | "csneg" => {
            expect(ops, 4, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            check_same_width(rd, rn, 1)?;
            check_same_width(rn, rm, 2)?;
            if rd.is_sp || rn.is_sp || rm.is_sp {
                return Err(EncError::new("conditional select does not accept SP"));
            }
            let c = cond(&ops[3], 3)?;
            let (op2, o2) = match m { "csel" => (0, 0), "csinc" => (0, 1), "csinv" => (1, 0), _ => (1, 1) };
            Ok(vec![insn_condsel(rd.width == 64, op2, o2, rm.num as u32, c as u32, rn.num as u32, rd.num as u32)])
        }
        "cset" | "csetm" => {
            expect(ops, 2, m)?;
            let rd = gpr(&ops[0], 0)?;
            if rd.is_sp {
                return Err(EncError::at("cset cannot target SP", 0));
            }
            let c = cond(&ops[1], 1)? ^ 1;
            let (op2, o2) = if m == "cset" { (0, 1) } else { (1, 0) };
            Ok(vec![insn_condsel(rd.width == 64, op2, o2, 31, c as u32, 31, rd.num as u32)])
        }
        "ccmp" | "ccmn" => enc_ccmp(m, ops, ctx),
        "mul" | "madd" | "msub" | "mneg" => enc_mul(m, ops, ctx),
        "smull" | "umull" | "smaddl" | "umaddl" | "smulh" | "umulh" => enc_mul_long(m, ops),
        "sdiv" | "udiv" => {
            expect(ops, 3, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            check_same_width(rd, rn, 1)?;
            check_same_width(rn, rm, 2)?;
            if rd.is_sp || rn.is_sp || rm.is_sp || is_zero_reg(rd) || is_zero_reg(rn) || is_zero_reg(rm) {
                return Err(EncError::new("division does not accept SP or the zero register"));
            }
            Ok(vec![insn_dp2(rd.width == 64, if m == "udiv" { 2 } else { 3 }, rm.num as u32, rn.num as u32, rd.num as u32)])
        }
        "adr" | "adrp" => enc_adr(m, ops, ctx),
        "crc32b" | "crc32h" | "crc32w" | "crc32x" | "crc32cb" | "crc32ch" | "crc32cw" | "crc32cx" => {
            enc_crc(m, ops)
        }
        _ => Err(EncError::new(format!("unhandled data-processing instruction '{}'", m))),
    }
}

fn enc_mov(ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    expect(ops, 2, "mov")?;
    let rd = gpr(&ops[0], 0)?;
    let src = ops[1].trim();
    if let Some(rn) = parse_gpr(src) {
        if rd.width != rn.width {
            return Err(EncError::at(format!("cannot move {} into {}", rn.text(), rd.text()), 1));
        }
        if rd.is_sp || rn.is_sp {
            // ADD (immediate) with #0 is the architectural SP move.
            return Ok(vec![insn_addsub_imm(rd.width == 64, false, false, false, 0, rn.num as u32, rd.num as u32)]);
        }
        if is_zero_reg(rd) || is_zero_reg(rn) {
            // ORR with the zero register is still a valid architectural move.
        }
        return Ok(vec![insn_logical_reg(rd.width == 64, 1, 0, false, rn.num as u32, 0, 31, rd.num as u32)]);
    }
    if src.starts_with('#') || src.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '(').unwrap_or(false) {
        if rd.is_sp {
            return Err(EncError::at("cannot move an immediate directly into SP; use movz + add", 1));
        }
        let v = ctx.imm(src, 1)?;
        let width = rd.width as u32;
        let mask: u64 = if width == 64 { u64::MAX } else { (1u64 << width) - 1 };
        if width == 32 && !(0..=0xffff_ffff).contains(&v) {
            return Err(EncError::at("immediate does not fit in 32 bits", 1));
        }
        let uv = (v as u64) & mask;
        if let Some((hw, imm16)) = movz_imm(uv, width) {
            return Ok(vec![insn_movwide(rd.width == 64, 2, hw, imm16 as u32, rd.num as u32)]);
        }
        if let Some((hw, imm16)) = movn_imm(uv, width) {
            return Ok(vec![insn_movwide(rd.width == 64, 0, hw, imm16 as u32, rd.num as u32)]);
        }
        if let Some((n, immr, imms)) = encode_logical_imm(uv, width) {
            return Ok(vec![insn_logical_imm(rd.width == 64, 1, n, immr, imms, 31, rd.num as u32)]);
        }
        return Err(EncError::at(
            "immediate cannot be encoded in one instruction (the assembler uses a literal pool for this form)",
            1,
        ));
    }
    Err(EncError::at(format!("expected a register or immediate, found '{}'", src), 1))
}

fn enc_movwide(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    expect(ops, 2, m)?;
    let rd = gpr(&ops[0], 0)?;
    if rd.is_sp {
        return Err(EncError::at("move-wide instructions cannot target SP", 0));
    }
    let (val, shift) = imm_shift(&ops[1], ctx, 1)?;
    if !(0..=0xffff).contains(&val) {
        return Err(EncError::at("immediate must be in the range 0..65535", 1));
    }
    let sh = shift.unwrap_or(0);
    if sh < 0 || sh % 16 != 0 || sh > 48 || (rd.width == 32 && sh > 16) {
        return Err(EncError::at("shift must be 0, 16, 32 or 48 and fit the register width", 1));
    }
    let opc = match m { "movn" => 0, "movz" => 2, _ => 3 };
    Ok(vec![insn_movwide(rd.width == 64, opc, (sh / 16) as u32, val as u32, rd.num as u32)])
}

fn enc_addsub(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    let sub = m == "sub" || m == "subs";
    let setflags = m == "adds" || m == "subs";
    enc_addsub_impl(sub, setflags, ops, ctx, false)
}

fn enc_addsub_impl(sub: bool, setflags: bool, ops: &[String], ctx: &Ctx, _alias: bool) -> EResult<Vec<u32>> {
    if ops.len() < 2 || ops.len() > 3 {
        return Err(EncError::new("add/subtract takes a destination, a source and a third operand"));
    }
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    check_same_width(rd, rn, 1)?;
    if setflags && rd.is_sp {
        return Err(EncError::at("the flag-setting form cannot write SP", 0));
    }
    let op2 = ops[2].trim();
    if let Some(rm) = parse_gpr(op2) {
        check_same_width(rn, rm, 2)?;
        if rn.is_sp || rd.is_sp || rm.is_sp {
            // Extended-register form is the SP-capable form.
            let option = if rn.width == 64 { 3 } else { 2 }; // UXTX / UXTW
            return Ok(vec![insn_addsub_ext(rd.width == 64, sub, setflags, rm.num as u32, option, 0, rn.num as u32, rd.num as u32)]);
        }
        if is_zero_reg(rd) || is_zero_reg(rn) || is_zero_reg(rm) {
            // allowed; xzr is a normal source/destination for arithmetic
        }
        return Ok(vec![insn_addsub_reg(rd.width == 64, sub, setflags, 0, 0, rm.num as u32, rn.num as u32, rd.num as u32)]);
    }
    if op2.starts_with('#') || op2.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '(').unwrap_or(false) {
        let (mut val, shift) = imm_shift(op2, ctx, 2)?;
        if rn.is_sp && setflags {
            // permitted
        }
        let mut sub = sub;
        if val < 0 {
            if val == i64::MIN {
                return Err(EncError::at("immediate out of range", 2));
            }
            val = -val;
            sub = !sub;
        }
        let max: i64 = 0xfff;
        let mut shift12 = false;
        let mut imm = val;
        if val > max {
            if val % 4096 == 0 && val / 4096 <= max {
                imm = val / 4096;
                shift12 = true;
            } else {
                return Err(EncError::at(
                    format!("immediate {} is out of range for add/sub (use movz/movk, or a shifted register)", val),
                    2,
                ));
            }
        }
        if let Some(sh) = shift {
            if sh != 12 || !shift12 {
                return Err(EncError::at("the only valid immediate shift is lsl #12", 2));
            }
        }
        return Ok(vec![insn_addsub_imm(rd.width == 64, sub, setflags, shift12, imm as u32, rn.num as u32, rd.num as u32)]);
    }
    // Register with an explicit shift or extend suffix.
    let (core, suffix) = split_suffix(op2);
    let rm = gpr(core, 2)?;
    if suffix.is_none() {
        check_same_width(rn, rm, 2)?;
    }
    if let Some((kind, amt)) = suffix {
        let k = kind.to_ascii_lowercase();
        if matches!(k.as_str(), "lsl" | "lsr" | "asr" | "ror") {
            check_same_width(rn, rm, 2)?;
            if rd.is_sp || rn.is_sp || rm.is_sp {
                return Err(EncError::at("shifted-register form does not accept SP", 2));
            }
            if k == "ror" {
                return Err(EncError::at("add/subtract does not support ror", 2));
            }
            let sh = match k.as_str() { "lsl" => 0, "lsr" => 1, _ => 2 };
            let amount = amt.unwrap_or(0);
            if amount < 0 || amount >= rn.width as i64 {
                return Err(EncError::at("shift amount out of range", 2));
            }
            return Ok(vec![insn_addsub_reg(rd.width == 64, sub, setflags, sh, amount as u32, rm.num as u32, rn.num as u32, rd.num as u32)]);
        }
        let option = match k.as_str() {
            "uxtb" => 0, "uxth" => 1, "uxtw" => 2, "uxtx" => 3,
            "sxtb" => 4, "sxth" => 5, "sxtw" => 6, "sxtx" => 7,
            _ => return Err(EncError::at(format!("unknown shift/extend '{}'", kind), 2)),
        };
        if rn.width == 32 && (option == 3 || option == 7) {
            return Err(EncError::at("uxtx/sxtx require 64-bit registers", 2));
        }
        if rn.width != rm.width && rn.width != 64 {
            return Err(EncError::at("extended register operands must be 64-bit with a W or X index", 2));
        }
        let amount = amt.unwrap_or(0);
        if !(0..=4).contains(&amount) {
            return Err(EncError::at("extend shift must be 0..4", 2));
        }
        return Ok(vec![insn_addsub_ext(rd.width == 64, sub, setflags, rm.num as u32, option, amount as u32, rn.num as u32, rd.num as u32)]);
    }
    Err(EncError::at(format!("expected a register or immediate, found '{}'", op2), 2))
}

fn enc_logical(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    if ops.len() < 2 || ops.len() > 3 {
        return Err(EncError::new(format!("'{}' takes a destination, a source and a third operand", m)));
    }
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    check_same_width(rd, rn, 1)?;
    if rd.is_sp || rn.is_sp {
        return Err(EncError::new("logical instructions do not accept SP"));
    }
    let (opc, invert) = match m {
        "and" => (0, false),
        "orr" => (1, false),
        "eor" => (2, false),
        "ands" => (3, false),
        "bic" => (0, true),
        "orn" => (1, true),
        "eon" => (2, true),
        "bics" => (3, true),
        _ => return Err(EncError::new(format!("unhandled logical '{}'", m))),
    };
    let op2 = ops[2].trim();
    if op2.starts_with('#') || op2.chars().next().map(|c| c.is_ascii_digit() || c == '(').unwrap_or(false) {
        if invert {
            return Err(EncError::at(format!("'{}' has no immediate form", m), 2));
        }
        let v = ctx.imm(op2, 2)?;
        let width = rd.width as u32;
        if width == 32 && !(0..=0xffff_ffff).contains(&v) {
            return Err(EncError::at("immediate does not fit in 32 bits", 2));
        }
        let mask: u64 = if width == 64 { u64::MAX } else { (1u64 << width) - 1 };
        let uv = (v as u64) & mask;
        match encode_logical_imm(uv, width) {
            Some((n, immr, imms)) => Ok(vec![insn_logical_imm(rd.width == 64, opc, n, immr, imms, rn.num as u32, rd.num as u32)]),
            None => Err(EncError::at(format!("0x{:x} is not a valid logical (bitmask) immediate", uv), 2)),
        }
    } else {
        let (core, suffix) = split_suffix(op2);
        let rm = gpr(core, 2)?;
        check_same_width(rn, rm, 2)?;
        if rm.is_sp {
            return Err(EncError::at("logical instructions do not accept SP", 2));
        }
        let (shift, amount) = match suffix {
            None => (0u32, 0i64),
            Some((kind, amt)) => {
                let k = kind.to_ascii_lowercase();
                let sh = match k.as_str() { "lsl" => 0, "lsr" => 1, "asr" => 2, "ror" => 3, _ => return Err(EncError::at("invalid shift for logical instruction", 2)) };
                (sh, amt.unwrap_or(0))
            }
        };
        if amount < 0 || amount >= rn.width as i64 {
            return Err(EncError::at("shift amount out of range", 2));
        }
        Ok(vec![insn_logical_reg(rd.width == 64, opc, shift, invert, rm.num as u32, amount as u32, rn.num as u32, rd.num as u32)])
    }
}

fn enc_shift_alias(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    if ops.len() < 2 || ops.len() > 3 {
        return Err(EncError::new(format!("'{}' takes a destination, a source and a shift amount", m)));
    }
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    check_same_width(rd, rn, 1)?;
    if rd.is_sp || rn.is_sp {
        return Err(EncError::new("shift instructions do not accept SP"));
    }
    let bits = rd.width as i64;
    let third = ops.get(2).map(|s| s.trim());
    let is_reg = third.map(|s| parse_gpr(s).is_some()).unwrap_or(false);
    if is_reg {
        if m == "ror" {
            let rm = gpr(third.unwrap(), 2)?;
            check_same_width(rn, rm, 2)?;
            return Ok(vec![insn_dp2(rd.width == 64, 11, rm.num as u32, rn.num as u32, rd.num as u32)]);
        }
        let rm = gpr(third.unwrap(), 2)?;
        check_same_width(rn, rm, 2)?;
        let opcode = match m { "lsl" => 8, "lsr" => 9, _ => 10 };
        return Ok(vec![insn_dp2(rd.width == 64, opcode, rm.num as u32, rn.num as u32, rd.num as u32)]);
    }
    let amount = match third {
        Some(s) => ctx.imm_u(s, 2, 6)? as i64,
        None => return Err(EncError::new(format!("'{}' requires a shift amount", m))),
    };
    if amount < 0 || amount >= bits {
        return Err(EncError::at(format!("shift amount must be 0..{}", bits - 1), 2));
    }
    let (opc, immr, imms) = match m {
        "lsl" => (2, ((-amount) % bits) as u32, (bits - 1 - amount) as u32),
        "lsr" => (2, amount as u32, (bits - 1) as u32),
        "asr" => (0, amount as u32, (bits - 1) as u32),
        _ => {
            // ROR alias of EXTR with Rn == Rm.
            let mut w = 0x1380_0000u32 | (rn.num as u32) << 16 | (rn.num as u32) << 5 | rd.num as u32;
            if rd.width == 64 { w |= 1 << 31; w |= 1 << 22; }
            w |= (amount as u32 & 63) << 10;
            return Ok(vec![w]);
        }
    };
    Ok(vec![insn_bitfield(rd.width == 64, opc, immr, imms, rn.num as u32, rd.num as u32)])
}

fn enc_bitfield_alias(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    expect(ops, 4, m)?;
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    check_same_width(rd, rn, 1)?;
    if rd.is_sp || rn.is_sp {
        return Err(EncError::new("bitfield instructions do not accept SP"));
    }
    let bits = rd.width as i64;
    let lsb = ctx.imm_u(&ops[2], 2, 6)? as i64;
    let width = ctx.imm_u(&ops[3], 3, 6)? as i64;
    if width <= 0 || lsb + width > bits {
        return Err(EncError::at("bitfield exceeds register width", 3));
    }
    let (opc, immr, imms) = match m {
        "ubfx" => (2, lsb, lsb + width - 1),
        "sbfx" => (0, lsb, lsb + width - 1),
        "bfxil" => (1, lsb, lsb + width - 1),
        "bfi" => (1, ((-lsb) % bits + bits) % bits, width - 1),
        "sbfiz" => (0, ((-lsb) % bits + bits) % bits, width - 1),
        "ubfiz" => (2, ((-lsb) % bits + bits) % bits, width - 1),
        _ => return Err(EncError::new("unhandled bitfield alias")),
    };
    Ok(vec![insn_bitfield(rd.width == 64, opc, immr as u32, imms as u32, rn.num as u32, rd.num as u32)])
}

fn enc_extend(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    expect(ops, 2, m)?;
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    if rn.width != 32 {
        return Err(EncError::at("source of a sign/zero extension must be a W register", 1));
    }
    if rd.is_sp || rn.is_sp {
        return Err(EncError::new("extension instructions do not accept SP"));
    }
    let (opc, imms, force32) = match m {
        "sxtb" => (0, 7, false),
        "sxth" => (0, 15, false),
        "sxtw" => (0, 31, false),
        "uxtb" => (2, 7, true),
        "uxth" => (2, 15, true),
        "uxtw" => (2, 31, false),
        _ => return Err(EncError::new("unhandled extension")),
    };
    if (m == "sxtw" || m == "uxtw") && rd.width != 64 {
        return Err(EncError::at(format!("'{}' requires a 64-bit destination", m), 0));
    }
    // UXTB/UXTH always operate on 32 bits, even with an X destination;
    // SXTB/SXTH with an X destination sign-extend across all 64 bits.
    let sf = if force32 { false } else { rd.width == 64 };
    Ok(vec![insn_bitfield(sf, opc, 0, imms, rn.num as u32, rd.num as u32)])
}

fn enc_ccmp(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    expect(ops, 4, m)?;
    let rn = gpr(&ops[0], 0)?;
    if rn.is_sp {
        return Err(EncError::at("conditional compare does not accept SP", 0));
    }
    let nzcv = ctx.imm_u(&ops[2], 2, 4)? as u32;
    let c = cond(&ops[3], 3)? as u32;
    let op2 = ops[1].trim();
    let sf = rn.width == 64;
    let mut w = match (m, sf) {
        ("ccmp", true) => 0xFA40_0000u32,
        ("ccmp", false) => 0x7A40_0000,
        ("ccmn", true) => 0xBA40_0000,
        _ => 0x3A40_0000,
    };
    if let Some(rm) = parse_gpr(op2) {
        check_same_width(rn, rm, 1)?;
        w |= (rm.num as u32) << 16;
    } else {
        let v = ctx.imm_u(op2, 1, 5)? as u32;
        w |= 0x800;
        w |= v << 16;
    }
    w |= c << 12;
    w |= (rn.num as u32) << 5;
    w |= nzcv;
    Ok(vec![w])
}

fn enc_mul(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    let _ = ctx;
    match m {
        "mul" | "mneg" => {
            expect(ops, 3, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            check_same_width(rd, rn, 1)?;
            check_same_width(rn, rm, 2)?;
            if rd.is_sp || rn.is_sp || rm.is_sp {
                return Err(EncError::new("multiply does not accept SP"));
            }
            Ok(vec![insn_madd(rd.width == 64, m == "mneg", rm.num as u32, 31, rn.num as u32, rd.num as u32)])
        }
        "madd" | "msub" => {
            expect(ops, 4, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            let ra = gpr(&ops[3], 3)?;
            check_same_width(rd, rn, 1)?;
            check_same_width(rn, rm, 2)?;
            check_same_width(rn, ra, 3)?;
            if rd.is_sp || rn.is_sp || rm.is_sp || ra.is_sp {
                return Err(EncError::new("multiply does not accept SP"));
            }
            Ok(vec![insn_madd(rd.width == 64, m == "msub", rm.num as u32, ra.num as u32, rn.num as u32, rd.num as u32)])
        }
        _ => Err(EncError::new("unhandled multiply")),
    }
}

fn enc_mul_long(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    match m {
        "smull" | "umull" => {
            expect(ops, 3, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            if rd.width != 64 || rn.width != 32 || rm.width != 32 {
                return Err(EncError::new("smull/umull require Xd, Wn, Wm"));
            }
            let base = if m == "smull" { 0x9B20_0000u32 } else { 0x9BA0_0000 };
            Ok(vec![base | (rm.num as u32) << 16 | 31 << 10 | (rn.num as u32) << 5 | rd.num as u32])
        }
        "smaddl" | "umaddl" => {
            expect(ops, 4, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            let ra = gpr(&ops[3], 3)?;
            if rd.width != 64 || rn.width != 32 || rm.width != 32 || ra.width != 64 {
                return Err(EncError::new("smaddl/umaddl require Xd, Wn, Wm, Xa"));
            }
            let base = if m == "smaddl" { 0x9B20_0000u32 } else { 0x9BA0_0000 };
            Ok(vec![base | (rm.num as u32) << 16 | (ra.num as u32) << 10 | (rn.num as u32) << 5 | rd.num as u32])
        }
        "smulh" | "umulh" => {
            expect(ops, 3, m)?;
            let rd = gpr(&ops[0], 0)?;
            let rn = gpr(&ops[1], 1)?;
            let rm = gpr(&ops[2], 2)?;
            if rd.width != 64 || rn.width != 64 || rm.width != 64 {
                return Err(EncError::new("smulh/umulh require 64-bit registers"));
            }
            let base = if m == "smulh" { 0x9B40_7C00u32 } else { 0x9BC0_7C00 };
            Ok(vec![base | (rm.num as u32) << 16 | (rn.num as u32) << 5 | rd.num as u32])
        }
        _ => Err(EncError::new("unhandled long multiply")),
    }
}

fn enc_adr(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    expect(ops, 2, m)?;
    let rd = gpr(&ops[0], 0)?;
    if rd.width != 64 {
        return Err(EncError::at("address formation requires a 64-bit destination", 0));
    }
    let target = (ctx.eval)(ops[1].trim()).map_err(|e| EncError::at(e, 1))?;
    if m == "adr" {
        let delta = target - ctx.pc as i64;
        if !(-(1 << 20)..(1 << 20)).contains(&delta) {
            return Err(EncError::at("adr target is out of range (±1 MiB)", 1));
        }
        Ok(vec![insn_adr(rd.num as u32, delta)])
    } else {
        let delta_pages = (target >> 12) - ((ctx.pc as i64) >> 12);
        if !(-(1 << 20)..(1 << 20)).contains(&delta_pages) {
            return Err(EncError::at("adrp target is out of range (±4 GiB)", 1));
        }
        Ok(vec![insn_adrp(rd.num as u32, delta_pages)])
    }
}

fn enc_crc(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    expect(ops, 3, m)?;
    let rd = gpr(&ops[0], 0)?;
    let rn = gpr(&ops[1], 1)?;
    let rm = gpr(&ops[2], 2)?;
    if rd.width != 32 || rn.width != 32 {
        return Err(EncError::new("CRC32 instructions use W registers for the destination and accumulator"));
    }
    let (opcode, sf) = match m {
        "crc32b" => (0x10, false),
        "crc32h" => (0x11, false),
        "crc32w" => (0x12, false),
        "crc32x" => (0x13, true),
        "crc32cb" => (0x14, false),
        "crc32ch" => (0x15, false),
        "crc32cw" => (0x16, false),
        _ => (0x17, true),
    };
    if m == "crc32x" || m == "crc32cx" {
        if rm.width != 64 {
            return Err(EncError::at("this CRC variant requires an X source register", 2));
        }
    } else if rm.width != 32 {
        return Err(EncError::at("this CRC variant requires a W source register", 2));
    }
    let mut w = 0x1AC0_0000u32 | (opcode & 0x1f) << 10;
    if sf { w |= 1 << 31; }
    w |= (rm.num as u32) << 16 | (rn.num as u32) << 5 | rd.num as u32;
    Ok(vec![w])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn no_eval(_: &str) -> Result<i64, String> { Err("no symbols".into()) }
    fn c() -> Ctx<'static> { Ctx { pc: 0, eval: &no_eval, pool_offset: None, section_base: 0 } }
    fn one(m: &str, ops: &[&str]) -> u32 {
        let v: Vec<String> = ops.iter().map(|s| s.to_string()).collect();
        encode(m, &v, &c()).unwrap_or_else(|e| panic!("{} {:?}: {:?}", m, ops, e))[0]
    }
    #[test]
    fn moves() {
        assert_eq!(one("mov", &["x0", "#42"]), 0xD280_0540);
        assert_eq!(one("mov", &["w0", "#0xffffffff"]), 0x1280_0000);
        assert_eq!(one("movz", &["x0", "#0x1234 lsl #16"]), 0xD2A2_4680);
        assert_eq!(one("movk", &["x0", "#0xabcd lsl #32"]), 0xF2D5_79A0);
        assert_eq!(one("mov", &["x0", "x1"]), 0xAA01_03E0);
        assert_eq!(one("mov", &["x0", "sp"]), 0x9100_03E0);
    }
    #[test]
    fn arithmetic() {
        assert_eq!(one("add", &["x0", "x1", "x2"]), 0x8B02_0020);
        assert_eq!(one("add", &["x0", "x1", "#0x1000"]), 0x9140_0420);
        assert_eq!(one("sub", &["w0", "w1", "#1"]), 0x5100_0420);
        assert_eq!(one("cmp", &["x0", "#10"]), 0xF100_281F);
        assert_eq!(one("neg", &["x0", "x1"]), 0xCB01_03E0);
        assert_eq!(one("add", &["x0", "sp", "x1"]), 0x8B21_63E0);
    }
    #[test]
    fn logicals() {
        assert_eq!(one("and", &["x0", "x1", "#0xff"]), 0x9240_1C20);
        assert_eq!(one("orr", &["x0", "xzr", "#0xffffffff"]), 0xB240_7FE0);
        assert_eq!(one("eor", &["w0", "w1", "w2 lsl #3"]), 0x4A02_0C20);
        assert_eq!(one("mvn", &["x0", "x1"]), 0xAA21_03E0);
        assert_eq!(one("tst", &["x0", "#1"]), 0xF240_001F);
    }
    #[test]
    fn shifts() {
        assert_eq!(one("lsl", &["x0", "x1", "#4"]), 0xD37C_EC20);
        assert_eq!(one("lsr", &["x0", "x1", "#4"]), 0xD344_FC20);
        assert_eq!(one("asr", &["w0", "w1", "#2"]), 0x1302_7C20);
        assert_eq!(one("lslv", &["x0", "x1", "x2"]), 0x9AC2_2020);
    }
    #[test]
    fn condsel() {
        assert_eq!(one("csel", &["x0", "x1", "x2", "eq"]), 0x9A82_0020);
        assert_eq!(one("cset", &["x0", "eq"]), 0x9A9F_17E0);
        assert_eq!(one("csetm", &["x0", "ne"]), 0xDA9F_03E0);
    }
    #[test]
    fn multiply() {
        assert_eq!(one("mul", &["x0", "x1", "x2"]), 0x9B02_7C20);
        assert_eq!(one("madd", &["x0", "x1", "x2", "x3"]), 0x9B02_0C20);
        assert_eq!(one("udiv", &["w0", "w1", "w2"]), 0x1AC2_0820);
    }
    #[test]
    fn bitfields() {
        assert_eq!(one("ubfx", &["x0", "x1", "#8", "#8"]), 0xD348_3C20);
        assert_eq!(one("sxtb", &["x0", "w1"]), 0x9340_1C20);
        assert_eq!(one("sxtw", &["x0", "w1"]), 0x9340_7C20);
    }
}
