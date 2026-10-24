//! System, hint, barrier, exception and system-register encoders.

use super::*;

fn opt_code(s: &str, op: usize) -> EResult<u32> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() {
        return Ok(15);
    }
    Ok(match t.as_str() {
        "sy" => 15,
        "st" => 14,
        "ld" => 13,
        "ish" => 11,
        "ishst" => 10,
        "ishld" => 9,
        "nsh" => 7,
        "nshst" => 6,
        "nshld" => 5,
        "osh" => 3,
        "oshst" => 2,
        "oshld" => 1,
        _ => return Err(EncError::at(format!("unknown barrier option '{}'", s), op)),
    })
}

fn imm16(ctx: &Ctx, s: &str, op: usize) -> EResult<u32> {
    let v = ctx.imm_u(s, op, 16)?;
    Ok(v as u32)
}

pub fn encode(m: &str, ops: &[String], ctx: &Ctx) -> EResult<Vec<u32>> {
    match m {
        "nop" => {
            if !ops.is_empty() { return Err(EncError::new("'nop' takes no operands")); }
            Ok(vec![0xD503_201F])
        }
        "yield" => Ok(vec![0xD503_203F]),
        "wfe" => Ok(vec![0xD503_205F]),
        "wfi" => Ok(vec![0xD503_207F]),
        "sev" => Ok(vec![0xD503_209F]),
        "sevl" => Ok(vec![0xD503_20BF]),
        "eret" => Ok(vec![0xD69F_03E0]),
        "drps" => Ok(vec![0xD6BF_03E0]),
        "clrex" => Ok(vec![0xD503_3F5F]),
        "hint" => {
            if ops.len() != 1 {
                return Err(EncError::new("'hint' takes one immediate"));
            }
            let v = ctx.imm_u(&ops[0], 0, 7)?;
            Ok(vec![0xD503_201F | (v as u32) << 5])
        }
        "bti" => {
            let code = match ops.first().map(|s| s.trim().to_ascii_lowercase()).as_deref() {
                None => 0x241F,
                Some("c") => 0x245F,
                Some("j") => 0x249F,
                Some("jc") | Some("cj") => 0x24DF,
                Some(other) => return Err(EncError::at(format!("invalid bti target '{}'", other), 0)),
            };
            Ok(vec![0xD503_0000 | code])
        }
        "paciasp" => Ok(vec![0xD503_233F]),
        "autiasp" => Ok(vec![0xD503_23BF]),
        "pacibsp" => Ok(vec![0xD503_237F]),
        "autibsp" => Ok(vec![0xD503_23FF]),
        "svc" | "brk" | "hlt" => {
            if ops.len() != 1 {
                return Err(EncError::new(format!("'{}' takes one 16-bit immediate", m)));
            }
            let v = imm16(ctx, &ops[0], 0)?;
            let base = match m { "svc" => 0xD400_0001u32, "brk" => 0xD420_0000, _ => 0xD440_0000 };
            Ok(vec![base | v << 5])
        }
        "dmb" | "dsb" => {
            let opt = if ops.is_empty() { 15 } else { opt_code(&ops[0], 0)? };
            let base = if m == "dmb" { 0xD503_30BFu32 } else { 0xD503_309F };
            Ok(vec![base | opt << 8])
        }
        "isb" => {
            let opt = if ops.is_empty() { 15 } else { opt_code(&ops[0], 0)? };
            Ok(vec![0xD503_30DF | opt << 8])
        }
        "mrs" | "msr" => enc_sysreg(m, ops),
        "udf" => {
            let v = if ops.is_empty() { 0 } else { imm16(ctx, &ops[0], 0)? };
            Ok(vec![v << 16])
        }
        _ => Err(EncError::new(format!("unhandled system instruction '{}'", m))),
    }
}

/// System register descriptors: (op0, op1, CRn, CRm, op2, readable, writable).
fn sysreg(name: &str) -> Option<(u32, u32, u32, u32, u32, bool, bool)> {
    Some(match name {
        "nzcv" => (3, 3, 4, 2, 0, true, true),
        "fpcr" => (3, 3, 4, 4, 0, true, true),
        "fpsr" => (3, 3, 4, 4, 1, true, true),
        "tpidr_el0" => (3, 3, 13, 0, 2, true, true),
        "tpidrro_el0" => (3, 3, 13, 0, 3, true, false),
        "cntvct_el0" => (3, 3, 14, 0, 2, true, false),
        "cntfrq_el0" => (3, 3, 14, 0, 0, true, false),
        "ctr_el0" => (3, 3, 0, 0, 1, true, false),
        "dczid_el0" => (3, 3, 0, 0, 7, true, false),
        "midr_el1" => (3, 0, 0, 0, 0, true, false),
        "currentel" => (3, 0, 4, 2, 2, true, false),
        "sp_el0" => (3, 0, 4, 1, 0, true, true),
        _ => return None,
    })
}

fn enc_sysreg(m: &str, ops: &[String]) -> EResult<Vec<u32>> {
    if ops.len() != 2 {
        return Err(EncError::new(format!("'{}' takes two operands", m)));
    }
    if m == "mrs" {
        let rt = gpr(&ops[0], 0)?;
        if rt.width != 64 {
            return Err(EncError::at("mrs requires an X register destination", 0));
        }
        let name = ops[1].trim().to_ascii_lowercase();
        let (op0, op1, crn, crm, op2, readable, _) = sysreg(&name)
            .ok_or_else(|| EncError::at(format!("system register '{}' is not supported by this assembler", name), 1))?;
        if !readable {
            return Err(EncError::at(format!("system register '{}' is not readable", name), 1));
        }
        let mut w = 0xD530_0000u32 | (op0 - 2) << 19 | op1 << 16 | crn << 12 | crm << 8 | op2 << 5;
        w |= rt.num as u32;
        Ok(vec![w])
    } else {
        let name = ops[0].trim().to_ascii_lowercase();
        let rt = gpr(&ops[1], 1)?;
        if rt.width != 64 {
            return Err(EncError::at("msr requires an X source register", 1));
        }
        let (op0, op1, crn, crm, op2, _, writable) = sysreg(&name)
            .ok_or_else(|| EncError::at(format!("system register '{}' is not supported by this assembler", name), 0))?;
        if !writable {
            return Err(EncError::at(format!("system register '{}' is read-only from EL0", name), 0));
        }
        let mut w = 0xD510_0000u32 | (op0 - 2) << 19 | op1 << 16 | crn << 12 | crm << 8 | op2 << 5;
        w |= rt.num as u32;
        Ok(vec![w])
    }
}
