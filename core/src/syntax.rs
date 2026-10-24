//! Shared lexical / semantic vocabulary for GNU-style AArch64 assembly.
//!
//! The instruction metadata table is deliberately hand written. It is the
//! single source of truth for highlighting, completion, the offline
//! instruction reference and the "is this mnemonic known?" check used by the
//! assembler. It only advertises instructions that the encoder in
//! `asm::encode` actually implements.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Instruction,
    Register,
    VectorRegister,
    LabelDecl,
    LabelRef,
    Immediate,
    Directive,
    Comment,
    String,
    Number,
    Operator,
    Condition,
    Invalid,
    Plain,
}

impl TokenKind {
    pub fn code(self) -> u8 {
        match self {
            TokenKind::Instruction => 1,
            TokenKind::Register => 2,
            TokenKind::VectorRegister => 3,
            TokenKind::LabelDecl => 4,
            TokenKind::LabelRef => 5,
            TokenKind::Immediate => 6,
            TokenKind::Directive => 7,
            TokenKind::Comment => 8,
            TokenKind::String => 9,
            TokenKind::Number => 10,
            TokenKind::Operator => 11,
            TokenKind::Condition => 12,
            TokenKind::Invalid => 13,
            TokenKind::Plain => 14,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Arithmetic,
    Logical,
    Move,
    Memory,
    Branch,
    Stack,
    Simd,
    Crypto,
    System,
    Armv9,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Arithmetic => "Arithmetic",
            Category::Logical => "Logical",
            Category::Move => "Move",
            Category::Memory => "Memory",
            Category::Branch => "Branch",
            Category::Stack => "Stack",
            Category::Simd => "SIMD / FP",
            Category::Crypto => "Crypto",
            Category::System => "System",
            Category::Armv9 => "ARMv9",
        }
    }
}

/// A mnemonic known to the assembler, with reference material.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct InsnInfo {
    pub mnemonic: &'static str,
    pub syntax: &'static str,
    pub description: &'static str,
    pub extension: &'static str,
    pub category: Category,
    pub example: &'static str,
    /// Encoding family shown in the instruction reference.
    pub encoding: &'static str,
    /// CPU feature required at run time, if any (`neon`, `sve`, ...).
    pub requires: Option<&'static str>,
}

macro_rules! info {
    ($m:expr, $s:expr, $d:expr, $e:expr, $c:expr, $ex:expr, $enc:expr) => {
        InsnInfo {
            mnemonic: $m,
            syntax: $s,
            description: $d,
            extension: $e,
            category: $c,
            example: $ex,
            encoding: $enc,
            requires: None,
        }
    };
    ($m:expr, $s:expr, $d:expr, $e:expr, $c:expr, $ex:expr, $enc:expr, $r:expr) => {
        InsnInfo {
            mnemonic: $m,
            syntax: $s,
            description: $d,
            extension: $e,
            category: $c,
            example: $ex,
            encoding: $enc,
            requires: Some($r),
        }
    };
}

/// Curated reference. Descriptions are original prose, written for this
/// project; they are not copied from the Arm Architecture Reference Manual.
pub static INSTRUCTIONS: &[InsnInfo] = &[
    info!("mov", "mov <Wd|Xd|SP>, <Wm|Xm|SP|#imm>", "Copy a register or immediate into a general-purpose register. Large immediates are materialised through a literal pool.", "Base (v8.0)", Category::Move, "mov x0, #42", "Move (register) / Move wide immediate"),
    info!("movz", "movz <Wd|Xd>, #imm16{, lsl #shift}", "Move a 16-bit immediate into a register, zeroing all other bits.", "Base (v8.0)", Category::Move, "movz x0, #0x1234, lsl #16", "Move wide immediate"),
    info!("movn", "movn <Wd|Xd>, #imm16{, lsl #shift}", "Move the bitwise inverse of a 16-bit immediate into a register.", "Base (v8.0)", Category::Move, "movn w0, #0", "Move wide immediate"),
    info!("movk", "movk <Wd|Xd>, #imm16{, lsl #shift}", "Insert a 16-bit immediate into one halfword of a register, keeping the other bits.", "Base (v8.0)", Category::Move, "movk x0, #0xabcd, lsl #32", "Move wide immediate"),
    info!("add", "add <Wd|Xd|SP>, <Wn|Xn|SP>, <Wm|Xm|SP|#imm>{, <shift>}", "Add two registers or a register and an immediate. The immediate form may be shifted left by 12.", "Base (v8.0)", Category::Arithmetic, "add x0, x1, #1", "Add/subtract immediate / shifted register"),
    info!("adds", "adds <Wd|Xd>, <Wn|Xn|SP>, <Wm|Xm|#imm>{, <shift>}", "Add and update the NZCV condition flags.", "Base (v8.0)", Category::Arithmetic, "adds x0, x0, #1", "Add/subtract immediate / shifted register"),
    info!("sub", "sub <Wd|Xd|SP>, <Wn|Xn|SP>, <Wm|Xm|SP|#imm>{, <shift>}", "Subtract a register or immediate from a register.", "Base (v8.0)", Category::Arithmetic, "sub sp, sp, #16", "Add/subtract immediate / shifted register"),
    info!("subs", "subs <Wd|Xd>, <Wn|Xn|SP>, <Wm|Xm|#imm>{, <shift>}", "Subtract and update the NZCV condition flags.", "Base (v8.0)", Category::Arithmetic, "subs w2, w0, w1", "Add/subtract immediate / shifted register"),
    info!("cmp", "cmp <Wn|Xn|SP>, <Wm|Xm|#imm>{, <shift>}", "Compare: subtract operands and set flags without writing a result.", "Base (v8.0)", Category::Arithmetic, "cmp x0, #10", "Subs alias"),
    info!("cmn", "cmn <Wn|Xn|SP>, <Wm|Xm|#imm>{, <shift>}", "Compare negative: add operands and set flags without writing a result.", "Base (v8.0)", Category::Arithmetic, "cmn x0, #1", "Adds alias"),
    info!("neg", "neg <Wd|Xd>, <Wm|Xm>{, <shift>}", "Negate a register (subtract it from zero).", "Base (v8.0)", Category::Arithmetic, "neg x0, x1", "Sub shifted register alias"),
    info!("negs", "negs <Wd|Xd>, <Wm|Xm>{, <shift>}", "Negate a register and set the condition flags.", "Base (v8.0)", Category::Arithmetic, "negs x0, x1", "Subs shifted register alias"),
    info!("mul", "mul <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Multiply two registers, keeping the low half of the product.", "Base (v8.0)", Category::Arithmetic, "mul x0, x1, x2", "Madd alias with XZR addend"),
    info!("madd", "madd <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <Wa|Xa>", "Multiply two registers and add a third: Rd = Rn * Rm + Ra.", "Base (v8.0)", Category::Arithmetic, "madd x0, x1, x2, x3", "Multiply-add"),
    info!("msub", "msub <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <Wa|Xa>", "Multiply two registers and subtract from a third: Rd = Ra - Rn * Rm.", "Base (v8.0)", Category::Arithmetic, "msub x0, x1, x2, x3", "Multiply-subtract"),
    info!("mneg", "mneg <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Multiply two registers and negate the product.", "Base (v8.0)", Category::Arithmetic, "mneg x0, x1, x2", "Msub alias with XZR addend"),
    info!("smull", "smull <Xd>, <Wn>, <Wm>", "Signed multiply producing a 64-bit result from 32-bit operands.", "Base (v8.0)", Category::Arithmetic, "smull x0, w1, w2", "Smaddl alias"),
    info!("umull", "umull <Xd>, <Wn>, <Wm>", "Unsigned multiply producing a 64-bit result from 32-bit operands.", "Base (v8.0)", Category::Arithmetic, "umull x0, w1, w2", "Umaddl alias"),
    info!("sdiv", "sdiv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Signed integer divide. Division by zero returns zero on Arm.", "Base (v8.0)", Category::Arithmetic, "sdiv w0, w1, w2", "Data-processing (2 source)"),
    info!("udiv", "udiv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Unsigned integer divide. Division by zero returns zero on Arm.", "Base (v8.0)", Category::Arithmetic, "udiv x0, x1, x2", "Data-processing (2 source)"),
    info!("and", "and <Wd|Xd>, <Wn|Xn>, <Wm|Xm|#imm>{, <shift>}", "Bitwise AND of a register and a register or logical immediate.", "Base (v8.0)", Category::Logical, "and x0, x1, #0xff", "Logical immediate / shifted register"),
    info!("orr", "orr <Wd|Xd>, <Wn|Xn>, <Wm|Xm|#imm>{, <shift>}", "Bitwise OR of a register and a register or logical immediate.", "Base (v8.0)", Category::Logical, "orr x0, x1, #1", "Logical immediate / shifted register"),
    info!("eor", "eor <Wd|Xd>, <Wn|Xn>, <Wm|Xm|#imm>{, <shift>}", "Bitwise exclusive OR.", "Base (v8.0)", Category::Logical, "eor x0, x0, x1", "Logical immediate / shifted register"),
    info!("ands", "ands <Wd|Xd>, <Wn|Xn>, <Wm|Xm|#imm>{, <shift>}", "Bitwise AND and update NZCV.", "Base (v8.0)", Category::Logical, "ands x0, x1, #1", "Logical immediate / shifted register"),
    info!("bic", "bic <Wd|Xd>, <Wn|Xn>, <Wm|Xm>{, <shift>}", "Bitwise AND with the inverse of the second register.", "Base (v8.0)", Category::Logical, "bic x0, x1, x2", "Logical shifted register"),
    info!("orn", "orn <Wd|Xd>, <Wn|Xn>, <Wm|Xm>{, <shift>}", "Bitwise OR with the inverse of the second register.", "Base (v8.0)", Category::Logical, "orn x0, x1, x2", "Logical shifted register"),
    info!("eon", "eon <Wd|Xd>, <Wn|Xn>, <Wm|Xm>{, <shift>}", "Bitwise exclusive OR with the inverse of the second register.", "Base (v8.0)", Category::Logical, "eon x0, x1, x2", "Logical shifted register"),
    info!("bics", "bics <Wd|Xd>, <Wn|Xn>, <Wm|Xm>{, <shift>}", "Bitwise AND with inverse and update NZCV.", "Base (v8.0)", Category::Logical, "bics x0, x1, x2", "Logical shifted register"),
    info!("mvn", "mvn <Wd|Xd>, <Wm|Xm>{, <shift>}", "Bitwise NOT of a register.", "Base (v8.0)", Category::Logical, "mvn x0, x1", "Orn alias with XZR"),
    info!("tst", "tst <Wn|Xn>, <Wm|Xm|#imm>{, <shift>}", "Test bits: AND operands and set flags without writing a result.", "Base (v8.0)", Category::Logical, "tst x0, #1", "Ands alias"),
    info!("lsl", "lsl <Wd|Xd>, <Wn|Xn>, <Wm|#imm>", "Logical shift left by an immediate or register amount.", "Base (v8.0)", Category::Logical, "lsl x0, x1, #4", "Ubfm / Lslv"),
    info!("lsr", "lsr <Wd|Xd>, <Wn|Xn>, <Wm|#imm>", "Logical shift right by an immediate or register amount.", "Base (v8.0)", Category::Logical, "lsr x0, x1, #4", "Ubfm / Lsrv"),
    info!("asr", "asr <Wd|Xd>, <Wn|Xn>, <Wm|#imm>", "Arithmetic shift right by an immediate or register amount.", "Base (v8.0)", Category::Logical, "asr w0, w1, #2", "Sbfm / Asrv"),
    info!("ror", "ror <Wd|Xd>, <Wn|Xn>, <Wm|#imm>", "Rotate right by an immediate or register amount.", "Base (v8.0)", Category::Logical, "ror x0, x1, #8", "Extr / Rorv"),
    info!("lslv", "lslv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Variable logical shift left.", "Base (v8.0)", Category::Logical, "lslv x0, x1, x2", "Data-processing (2 source)"),
    info!("lsrv", "lsrv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Variable logical shift right.", "Base (v8.0)", Category::Logical, "lsrv x0, x1, x2", "Data-processing (2 source)"),
    info!("asrv", "asrv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Variable arithmetic shift right.", "Base (v8.0)", Category::Logical, "asrv x0, x1, x2", "Data-processing (2 source)"),
    info!("rorv", "rorv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>", "Variable rotate right.", "Base (v8.0)", Category::Logical, "rorv x0, x1, x2", "Data-processing (2 source)"),
    info!("ubfm", "ubfm <Wd|Xd>, <Wn|Xn>, #immr, #imms", "Unsigned bitfield move (the basis of ubfx and uxtb).", "Base (v8.0)", Category::Logical, "ubfm x0, x1, #0, #7", "Bitfield"),
    info!("sbfm", "sbfm <Wd|Xd>, <Wn|Xn>, #immr, #imms", "Signed bitfield move (the basis of sbfx and sxtb).", "Base (v8.0)", Category::Logical, "sbfm x0, x1, #0, #7", "Bitfield"),
    info!("bfm", "bfm <Wd|Xd>, <Wn|Xn>, #immr, #imms", "Bitfield move, keeping bits outside the field.", "Base (v8.0)", Category::Logical, "bfm x0, x1, #8, #15", "Bitfield"),
    info!("ubfx", "ubfx <Wd|Xd>, <Wn|Xn>, #lsb, #width", "Extract an unsigned bitfield.", "Base (v8.0)", Category::Logical, "ubfx x0, x1, #8, #8", "Ubfm alias"),
    info!("sbfx", "sbfx <Wd|Xd>, <Wn|Xn>, #lsb, #width", "Extract a signed bitfield.", "Base (v8.0)", Category::Logical, "sbfx x0, x1, #8, #8", "Sbfm alias"),
    info!("bfi", "bfi <Wd|Xd>, <Wn|Xn>, #lsb, #width", "Insert a bitfield, keeping surrounding bits.", "Base (v8.0)", Category::Logical, "bfi x0, x1, #8, #8", "Bfm alias"),
    info!("bfxil", "bfxil <Wd|Xd>, <Wn|Xn>, #lsb, #width", "Extract a bitfield into the low bits, keeping the destination's high bits.", "Base (v8.0)", Category::Logical, "bfxil x0, x1, #8, #8", "Bfm alias"),
    info!("sxtb", "sxtb <Wd|Xd>, <Wm>", "Sign-extend a byte.", "Base (v8.0)", Category::Logical, "sxtb x0, w1", "Sbfm alias"),
    info!("sxth", "sxth <Wd|Xd>, <Wm>", "Sign-extend a halfword.", "Base (v8.0)", Category::Logical, "sxth x0, w1", "Sbfm alias"),
    info!("sxtw", "sxtw <Xd>, <Wm>", "Sign-extend a word to 64 bits.", "Base (v8.0)", Category::Logical, "sxtw x0, w1", "Sbfm alias"),
    info!("uxtb", "uxtb <Wd|Xd>, <Wm>", "Zero-extend a byte.", "Base (v8.0)", Category::Logical, "uxtb w0, w1", "Ubfm alias"),
    info!("uxth", "uxth <Wd|Xd>, <Wm>", "Zero-extend a halfword.", "Base (v8.0)", Category::Logical, "uxth w0, w1", "Ubfm alias"),
    info!("uxtw", "uxtw <Xd>, <Wm>", "Zero-extend a word to 64 bits.", "Base (v8.0)", Category::Logical, "uxtw x0, w1", "Ubfm alias"),
    info!("rev", "rev <Wd|Xd>, <Wn|Xn>", "Reverse the byte order of a register.", "Base (v8.0)", Category::Logical, "rev x0, x1", "Data-processing (1 source)"),
    info!("rev16", "rev16 <Wd|Xd>, <Wn|Xn>", "Reverse bytes within each halfword.", "Base (v8.0)", Category::Logical, "rev16 w0, w1", "Data-processing (1 source)"),
    info!("rev32", "rev32 <Xd>, <Xn>", "Reverse bytes within each word.", "Base (v8.0)", Category::Logical, "rev32 x0, x1", "Data-processing (1 source)"),
    info!("rbit", "rbit <Wd|Xd>, <Wn|Xn>", "Reverse the bit order of a register.", "Base (v8.0)", Category::Logical, "rbit x0, x1", "Data-processing (1 source)"),
    info!("clz", "clz <Wd|Xd>, <Wn|Xn>", "Count leading zero bits.", "Base (v8.0)", Category::Logical, "clz x0, x1", "Data-processing (1 source)"),
    info!("cls", "cls <Wd|Xd>, <Wn|Xn>", "Count leading sign bits.", "Base (v8.0)", Category::Logical, "cls w0, w1", "Data-processing (1 source)"),
    info!("csel", "csel <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <cond>", "Conditionally select the first or second source register.", "Base (v8.0)", Category::Arithmetic, "csel x0, x1, x2, eq", "Conditional select"),
    info!("csinc", "csinc <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <cond>", "Conditionally select, incrementing the second operand when the condition is false.", "Base (v8.0)", Category::Arithmetic, "csinc x0, x1, x2, eq", "Conditional select"),
    info!("csinv", "csinv <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <cond>", "Conditionally select, inverting the second operand when the condition is false.", "Base (v8.0)", Category::Arithmetic, "csinv x0, x1, x2, eq", "Conditional select"),
    info!("csneg", "csneg <Wd|Xd>, <Wn|Xn>, <Wm|Xm>, <cond>", "Conditionally select, negating the second operand when the condition is false.", "Base (v8.0)", Category::Arithmetic, "csneg x0, x1, x2, eq", "Conditional select"),
    info!("cset", "cset <Wd|Xd>, <cond>", "Set a register to 1 when the condition holds, otherwise 0.", "Base (v8.0)", Category::Arithmetic, "cset x0, eq", "Csinc alias"),
    info!("csetm", "csetm <Wd|Xd>, <cond>", "Set a register to all ones when the condition holds, otherwise 0.", "Base (v8.0)", Category::Arithmetic, "csetm x0, eq", "Csinv alias"),
    info!("ccmp", "ccmp <Wn|Xn>, <Wm|Xm|#imm>, #nzcv, <cond>", "Conditional compare, setting NZCV when the condition holds.", "Base (v8.0)", Category::Arithmetic, "ccmp x0, x1, #0, eq", "Conditional compare"),
    info!("ccmn", "ccmn <Wn|Xn>, <Wm|Xm|#imm>, #nzcv, <cond>", "Conditional compare negative.", "Base (v8.0)", Category::Arithmetic, "ccmn x0, #1, #0, eq", "Conditional compare"),
    info!("adr", "adr <Xd>, <label>", "Form the PC-relative address of a label within ±1 MiB.", "Base (v8.0)", Category::Move, "adr x0, message", "PC-relative address"),
    info!("adrp", "adrp <Xd>, <label>", "Form the address of the 4 KiB page containing a label within ±4 GiB.", "Base (v8.0)", Category::Move, "adrp x0, buffer", "PC-relative address"),
    info!("ldr", "ldr <Wt|Xt|Qt>, <[Xn{, #imm}]|label|=[expr]>", "Load a register from memory. Supports unsigned, pre/post-indexed, register-offset and literal forms.", "Base (v8.0)", Category::Memory, "ldr x0, [x1, #8]", "Load register"),
    info!("str", "str <Wt|Xt|Qt>, <[Xn{, #imm}]>", "Store a register to memory.", "Base (v8.0)", Category::Memory, "str x0, [sp, #16]", "Store register"),
    info!("ldrb", "ldrb <Wt|Xt>, <[Xn{, #imm}]>", "Load a byte and zero-extend it.", "Base (v8.0)", Category::Memory, "ldrb w0, [x1]", "Load register byte"),
    info!("strb", "strb <Wt>, <[Xn{, #imm}]>", "Store the low byte of a register.", "Base (v8.0)", Category::Memory, "strb w0, [x1]", "Store register byte"),
    info!("ldrh", "ldrh <Wt|Xt>, <[Xn{, #imm}]>", "Load a halfword and zero-extend it.", "Base (v8.0)", Category::Memory, "ldrh w0, [x1]", "Load register halfword"),
    info!("strh", "strh <Wt>, <[Xn{, #imm}]>", "Store the low halfword of a register.", "Base (v8.0)", Category::Memory, "strh w0, [x1]", "Store register halfword"),
    info!("ldrsw", "ldrsw <Xt>, <[Xn{, #imm}]|label>", "Load a 32-bit word and sign-extend it to 64 bits.", "Base (v8.0)", Category::Memory, "ldrsw x0, [x1]", "Load register signed word"),
    info!("ldrsb", "ldrsb <Wt|Xt>, <[Xn{, #imm}]>", "Load a byte and sign-extend it.", "Base (v8.0)", Category::Memory, "ldrsb x0, [x1]", "Load register signed byte"),
    info!("ldrsh", "ldrsh <Wt|Xt>, <[Xn{, #imm}]>", "Load a halfword and sign-extend it.", "Base (v8.0)", Category::Memory, "ldrsh x0, [x1]", "Load register signed halfword"),
    info!("ldp", "ldp <Wt|Xt>, <Wt2|Xt2>, <[Xn{, #imm}]>", "Load a pair of registers from consecutive memory locations.", "Base (v8.0)", Category::Memory, "ldp x0, x1, [sp], #16", "Load pair"),
    info!("stp", "stp <Wt|Xt>, <Wt2|Xt2>, <[Xn{, #imm}]>", "Store a pair of registers to consecutive memory locations.", "Base (v8.0)", Category::Stack, "stp x29, x30, [sp, #-16]!", "Store pair"),
    info!("ldar", "ldar <Wt|Xt>, [Xn]", "Load with acquire semantics.", "Base (v8.0)", Category::Memory, "ldar x0, [x1]", "Load-acquire"),
    info!("stlr", "stlr <Wt|Xt>, [Xn]", "Store with release semantics.", "Base (v8.0)", Category::Memory, "stlr x0, [x1]", "Store-release"),
    info!("ldxr", "ldxr <Wt|Xt>, [Xn]", "Load exclusive, marking the address for an exclusive store.", "Base (v8.0)", Category::Memory, "ldxr x0, [x1]", "Load exclusive"),
    info!("stxr", "stxr <Ws>, <Wt|Xt>, [Xn]", "Store exclusive; writes 0 to Ws on success, 1 on failure.", "Base (v8.0)", Category::Memory, "stxr w2, x0, [x1]", "Store exclusive"),
    info!("ldaxr", "ldaxr <Wt|Xt>, [Xn]", "Load exclusive with acquire semantics.", "Base (v8.0)", Category::Memory, "ldaxr x0, [x1]", "Load exclusive"),
    info!("stlxr", "stlxr <Ws>, <Wt|Xt>, [Xn]", "Store exclusive with release semantics.", "Base (v8.0)", Category::Memory, "stlxr w2, x0, [x1]", "Store exclusive"),
    info!("prfm", "prfm <prfop>, [Xn{, #imm}]", "Prefetch memory into a cache level.", "Base (v8.0)", Category::Memory, "prfm pldl1keep, [x0]", "Prefetch memory"),
    info!("b", "b <label>", "Unconditional branch within ±128 MiB.", "Base (v8.0)", Category::Branch, "b loop", "Unconditional branch immediate"),
    info!("bl", "bl <label>", "Branch with link, setting X30 to the return address.", "Base (v8.0)", Category::Branch, "bl function", "Branch with link"),
    info!("br", "br <Xn>", "Branch to the address in a register.", "Base (v8.0)", Category::Branch, "br x0", "Unconditional branch register"),
    info!("blr", "blr <Xn>", "Branch with link to the address in a register.", "Base (v8.0)", Category::Branch, "blr x0", "Branch with link register"),
    info!("ret", "ret {Xn}", "Return to the address in X30 (or another register).", "Base (v8.0)", Category::Branch, "ret", "Return"),
    info!("cbz", "cbz <Wt|Xt>, <label>", "Branch if a register is zero.", "Base (v8.0)", Category::Branch, "cbz x0, done", "Compare and branch"),
    info!("cbnz", "cbnz <Wt|Xt>, <label>", "Branch if a register is non-zero.", "Base (v8.0)", Category::Branch, "cbnz x0, loop", "Compare and branch"),
    info!("tbz", "tbz <Wt|Xt>, #bit, <label>", "Test one bit and branch if it is zero.", "Base (v8.0)", Category::Branch, "tbz w0, #0, even", "Test and branch"),
    info!("tbnz", "tbnz <Wt|Xt>, #bit, <label>", "Test one bit and branch if it is non-zero.", "Base (v8.0)", Category::Branch, "tbnz w0, #0, odd", "Test and branch"),
    info!("b.eq", "b.<cond> <label>", "Branch when the condition code holds. Condition codes: eq ne cs hs cc lo mi pl vs vc hi ls ge lt gt le al nv.", "Base (v8.0)", Category::Branch, "b.eq done", "Conditional branch immediate"),
    info!("nop", "nop", "No operation.", "Base (v8.0)", Category::System, "nop", "Hint"),
    info!("hint", "hint #imm", "Architectural hint instruction.", "Base (v8.0)", Category::System, "hint #0", "Hint"),
    info!("yield", "yield", "Yield hint to the scheduler.", "Base (v8.0)", Category::System, "yield", "Hint"),
    info!("wfe", "wfe", "Wait for an event.", "Base (v8.0)", Category::System, "wfe", "Hint"),
    info!("wfi", "wfi", "Wait for an interrupt.", "Base (v8.0)", Category::System, "wfi", "Hint"),
    info!("sev", "sev", "Send an event to other cores.", "Base (v8.0)", Category::System, "sev", "Hint"),
    info!("sevl", "sevl", "Send a local event.", "Base (v8.0)", Category::System, "sevl", "Hint"),
    info!("bti", "bti {c|j|jc}", "Branch target identification marker. Requires BTI to be enabled by the OS.", "v8.5 BTI", Category::Armv9, "bti c", "Hint", "bti"),
    info!("svc", "svc #imm16", "Supervisor call. On Android this enters the kernel; a raw SVC usually terminates the worker process.", "Base (v8.0)", Category::System, "svc #0", "Exception generation"),
    info!("brk", "brk #imm16", "Breakpoint. Deliberately raises a debug exception.", "Base (v8.0)", Category::System, "brk #0", "Exception generation"),
    info!("hlt", "hlt #imm16", "Halting breakpoint.", "Base (v8.0)", Category::System, "hlt #0", "Exception generation"),
    info!("dmb", "dmb {option}", "Data memory barrier.", "Base (v8.0)", Category::System, "dmb ish", "Data barrier"),
    info!("dsb", "dsb {option}", "Data synchronisation barrier.", "Base (v8.0)", Category::System, "dsb sy", "Data barrier"),
    info!("isb", "isb {option}", "Instruction synchronisation barrier.", "Base (v8.0)", Category::System, "isb", "Instruction barrier"),
    info!("mrs", "mrs <Xt>, <systemreg>", "Read a system register. Supported: nzcv, fpcr, tpidr_el0, cntvct_el0, cntfrq_el0, midr_el1 (where readable).", "Base (v8.0)", Category::System, "mrs x0, nzcv", "System register move"),
    info!("msr", "msr <systemreg>, <Xt|#imm>", "Write a system register. Supported: nzcv, fpcr, tpidr_el0.", "Base (v8.0)", Category::System, "msr nzcv, x0", "System register move"),
    info!("paciasp", "paciasp", "Sign the link register with the A key (pointer authentication).", "v8.3 PAC", Category::Armv9, "paciasp", "Hint", "paca"),
    info!("autiasp", "autiasp", "Authenticate the link register with the A key.", "v8.3 PAC", Category::Armv9, "autiasp", "Hint", "paca"),
    info!("pacibsp", "pacibsp", "Sign the link register with the B key.", "v8.3 PAC", Category::Armv9, "pacibsp", "Hint", "pacg"),
    info!("autibsp", "autibsp", "Authenticate the link register with the B key.", "v8.3 PAC", Category::Armv9, "autibsp", "Hint", "pacg"),
    info!("addv", "addv <Vd>.<T>, <Vn>.<T>", "Add all elements of a vector into a scalar.", "Base (v8.0) ASIMD", Category::Simd, "addv s0, v1.4s", "Advanced SIMD reduction", "neon"),
    info!("fadd", "fadd <Vd>.<T>, <Vn>.<T>, <Vm>.<T>", "Floating-point vector add (4S or 2D).", "Base (v8.0) ASIMD", Category::Simd, "fadd v0.4s, v1.4s, v2.4s", "Advanced SIMD three same", "neon"),
    info!("fsub", "fsub <Vd>.<T>, <Vn>.<T>, <Vm>.<T>", "Floating-point vector subtract (4S or 2D).", "Base (v8.0) ASIMD", Category::Simd, "fsub v0.4s, v1.4s, v2.4s", "Advanced SIMD three same", "neon"),
    info!("fmul", "fmul <Vd>.<T>, <Vn>.<T>, <Vm>.<T>", "Floating-point vector multiply (4S or 2D).", "Base (v8.0) ASIMD", Category::Simd, "fmul v0.4s, v1.4s, v2.4s", "Advanced SIMD three same", "neon"),
    info!("fdiv", "fdiv <Vd>.<T>, <Vn>.<T>, <Vm>.<T>", "Floating-point vector divide (4S or 2D).", "Base (v8.0) ASIMD", Category::Simd, "fdiv v0.4s, v1.4s, v2.4s", "Advanced SIMD three same", "neon"),
    info!("movi", "movi <Vd>.<T>, #imm{, lsl #shift}", "Move an immediate into every vector element.", "Base (v8.0) ASIMD", Category::Simd, "movi v0.16b, #0", "Advanced SIMD modified immediate", "neon"),
    info!("dup", "dup <Vd>.<T>, <Wn|Vn>.<T>[index]", "Broadcast a general-purpose register or vector element.", "Base (v8.0) ASIMD", Category::Simd, "dup v0.4s, w1", "Advanced SIMD copy", "neon"),
    info!("ins", "ins <Vd>.<T>[index], <Wn>", "Insert a general-purpose register into a vector element.", "Base (v8.0) ASIMD", Category::Simd, "ins v0.s[0], w1", "Advanced SIMD copy", "neon"),
    info!("umov", "umov <Wd>, <Vn>.<T>[index]", "Move a vector element to a general-purpose register.", "Base (v8.0) ASIMD", Category::Simd, "umov w0, v1.s[0]", "Advanced SIMD copy", "neon"),
    info!("ld1", "ld1 {<Vt>.<T>}, [<Xn>]{, #imm}", "Load one vector register from memory.", "Base (v8.0) ASIMD", Category::Simd, "ld1 {v0.16b}, [x1]", "Advanced SIMD structure load", "neon"),
    info!("st1", "st1 {<Vt>.<T>}, [<Xn>]{, #imm}", "Store one vector register to memory.", "Base (v8.0) ASIMD", Category::Simd, "st1 {v0.16b}, [x1]", "Advanced SIMD structure store", "neon"),
    info!("ld1r", "ld1r {<Vt>.<T>}, [<Xn>]", "Load one element and replicate it across the vector.", "Base (v8.0) ASIMD", Category::Simd, "ld1r {v0.4s}, [x1]", "Advanced SIMD structure load", "neon"),
    info!("aese", "aese <Vd>.16b, <Vn>.16b", "AES single round encryption. Requires the AES extension.", "Base (v8.0) Crypto", Category::Crypto, "aese v0.16b, v1.16b", "Advanced SIMD crypto", "aes"),
    info!("aesd", "aesd <Vd>.16b, <Vn>.16b", "AES single round decryption. Requires the AES extension.", "Base (v8.0) Crypto", Category::Crypto, "aesd v0.16b, v1.16b", "Advanced SIMD crypto", "aes"),
    info!("sha256h", "sha256h <Qd>, <Qn>, <Vm>.4s", "SHA-256 hash update part 1. Requires the SHA2 extension.", "Base (v8.0) Crypto", Category::Crypto, "sha256h q0, q1, v2.4s", "Advanced SIMD crypto", "sha2"),
    info!("pmull", "pmull <Vd>.8h, <Vn>.8b, <Vm>.8b", "Polynomial multiply (long). Requires the PMULL extension.", "Base (v8.0) Crypto", Category::Crypto, "pmull v0.8h, v1.8b, v2.8b", "Advanced SIMD crypto", "pmull"),
    info!("crc32b", "crc32b <Wd>, <Wn>, <Wm>", "CRC-32 byte. Requires the CRC32 extension.", "Base (v8.0) CRC", Category::Crypto, "crc32b w0, w1, w2", "CRC32", "crc32"),
];

pub fn lookup_instruction(mnemonic: &str) -> Option<&'static InsnInfo> {
    let lower = mnemonic.to_ascii_lowercase();
    INSTRUCTIONS.iter().find(|i| i.mnemonic == lower)
}

/// Complete list of mnemonics accepted by the parser, including aliases that
/// are implemented as pseudo-instructions but are not all in the reference
/// table above (they share an entry).
pub static MNEMONICS: &[&str] = &[
    "mov", "movz", "movn", "movk", "add", "adds", "sub", "subs", "cmp", "cmn", "neg", "negs",
    "mul", "madd", "msub", "mneg", "smull", "umull", "smaddl", "umaddl", "smulh", "umulh",
    "sdiv", "udiv", "and", "orr", "eor", "ands", "bic", "orn", "eon", "bics", "mvn", "tst",
    "lsl", "lsr", "asr", "ror", "lslv", "lsrv", "asrv", "rorv", "ubfm", "sbfm", "bfm", "ubfx",
    "sbfx", "bfi", "bfxil", "sbfiz", "ubfiz", "sxtb", "sxth", "sxtw", "uxtb", "uxth", "uxtw",
    "not", "rev", "rev16", "rev32", "rbit", "clz", "cls", "csel", "csinc", "csinv", "csneg", "cset",
    "csetm", "ccmp", "ccmn", "adr", "adrp", "ldr", "str", "ldrb", "strb", "ldrh", "strh",
    "ldrsb", "ldrsh", "ldrsw", "ldp", "stp", "ldur", "stur", "ldurb", "sturb", "ldurh", "sturh",
    "ldursb", "ldursh", "ldursw", "ldar", "stlr", "ldxr", "stxr", "ldaxr", "stlxr", "prfm",
    "b", "bl", "br", "blr", "ret", "cbz", "cbnz", "tbz", "tbnz", "nop", "hint", "yield", "wfe",
    "wfi", "sev", "sevl", "bti", "svc", "brk", "hlt", "dmb", "dsb", "isb", "mrs", "msr",
    "paciasp", "autiasp", "pacibsp", "autibsp", "eret", "drps", "clrex", "udf",
    "addv", "addp", "fadd", "fsub", "fmul", "fdiv", "fabs", "fneg", "fsqrt", "fcvt", "scvtf",
    "ucvtf", "fcvtzs", "fcvtzu", "fcmp", "fcmpe", "fmov", "movi", "mvni", "dup", "ins", "umov",
    "smov", "ld1", "st1", "ld1r", "ld2", "st2", "ld3", "st3", "ld4", "st4", "aese", "aesd",
    "aesmc", "aesimc", "sha256h", "sha256h2", "sha256su0", "sha256su1", "pmull", "pmull2",
    "crc32b", "crc32h", "crc32w", "crc32x", "crc32cb", "crc32ch", "crc32cw", "crc32cx",
    "sve", "sve2", "sme",
];

pub fn is_known_mnemonic(m: &str) -> bool {
    let lower = m.to_ascii_lowercase();
    MNEMONICS.contains(&lower.as_str())
}

pub static CONDITIONS: &[(&str, u8)] = &[
    ("eq", 0), ("ne", 1), ("cs", 2), ("hs", 2), ("cc", 3), ("lo", 3),
    ("mi", 4), ("pl", 5), ("vs", 6), ("vc", 7), ("hi", 8), ("ls", 9),
    ("ge", 10), ("lt", 11), ("gt", 12), ("le", 13), ("al", 14), ("nv", 15),
];

pub fn condition_code(s: &str) -> Option<u8> {
    let lower = s.to_ascii_lowercase();
    CONDITIONS.iter().find(|(n, _)| *n == lower).map(|(_, c)| *c)
}

pub fn condition_name(code: u8) -> &'static str {
    match code & 0xf {
        0 => "eq", 1 => "ne", 2 => "cs", 3 => "cc", 4 => "mi", 5 => "pl",
        6 => "vs", 7 => "vc", 8 => "hi", 9 => "ls", 10 => "ge", 11 => "lt",
        12 => "gt", 13 => "le", 14 => "al", _ => "nv",
    }
}

/// General-purpose register names accepted in source.
pub fn parse_gpr(s: &str) -> Option<Gpr> {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "sp" => return Some(Gpr { num: 31, width: 64, is_sp: true }),
        "wsp" => return Some(Gpr { num: 31, width: 32, is_sp: true }),
        "xzr" => return Some(Gpr { num: 31, width: 64, is_sp: false }),
        "wzr" => return Some(Gpr { num: 31, width: 32, is_sp: false }),
        "lr" => return Some(Gpr { num: 30, width: 64, is_sp: false }),
        "fp" => return Some(Gpr { num: 29, width: 64, is_sp: false }),
        _ => {}
    }
    let bytes = lower.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let width = match bytes[0] {
        b'x' => 64,
        b'w' => 32,
        _ => return None,
    };
    let num: u8 = lower[1..].parse().ok()?;
    if num > 30 {
        return None;
    }
    Some(Gpr { num, width, is_sp: false })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gpr {
    pub num: u8,
    pub width: u8,
    pub is_sp: bool,
}

impl Gpr {
    pub fn is_zero(&self) -> bool {
        self.num == 31 && !self.is_sp
    }
    pub fn text(&self) -> String {
        if self.is_sp {
            return if self.width == 64 { "sp".into() } else { "wsp".into() };
        }
        if self.num == 31 {
            return if self.width == 64 { "xzr".into() } else { "wzr".into() };
        }
        format!("{}{}", if self.width == 64 { 'x' } else { 'w' }, self.num)
    }
}

/// Vector / SVE / SME register with optional arrangement or element index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VReg {
    pub class: VClass,
    pub num: u8,
    pub q: bool,
    pub arr: Option<Arrangement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VClass {
    V,
    Q,
    D,
    S,
    H,
    B,
    Z,
    P,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrangement {
    pub count: u8,
    pub elem: Elem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elem {
    B,
    H,
    S,
    D,
    Q,
}

impl Elem {
    pub fn size(self) -> u8 {
        match self {
            Elem::B => 8,
            Elem::H => 16,
            Elem::S => 32,
            Elem::D => 64,
            Elem::Q => 128,
        }
    }
    pub fn ch(self) -> char {
        match self {
            Elem::B => 'b',
            Elem::H => 'h',
            Elem::S => 's',
            Elem::D => 'd',
            Elem::Q => 'q',
        }
    }
}

/// Parse `v0`, `v0.16b`, `v0.s[3]`, `q1`, `d2`, `z0.s`, `p0/z`.
/// Returns the register and the optional element index.
pub fn parse_vreg(s: &str) -> Option<(VReg, Option<u8>)> {
    let lower = s.trim().to_ascii_lowercase();
    let (base, index) = if let Some(open) = lower.find('[') {
        let close = lower.rfind(']')?;
        let idx: u8 = lower[open + 1..close].trim().parse().ok()?;
        (&lower[..open], Some(idx))
    } else {
        (lower.as_str(), None)
    };
    let (head, arr_str) = match base.find('.') {
        Some(p) => (&base[..p], Some(&base[p + 1..])),
        None => (base, None),
    };
    let bytes = head.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let (class, num_start) = match bytes[0] {
        b'v' => (VClass::V, 1),
        b'q' => (VClass::Q, 1),
        b'd' => (VClass::D, 1),
        b's' => (VClass::S, 1),
        b'h' => (VClass::H, 1),
        b'b' => (VClass::B, 1),
        b'z' => (VClass::Z, 1),
        b'p' => (VClass::P, 1),
        _ => return None,
    };
    if num_start >= bytes.len() || !bytes[num_start].is_ascii_digit() {
        return None;
    }
    let num: u8 = head[num_start..].parse().ok()?;
    let max = if class == VClass::P { 15 } else { 31 };
    if num > max {
        return None;
    }
    let arr = arr_str.and_then(|a| {
        parse_arrangement(a).or_else(|| match a {
            "b" => Some(Arrangement { count: 0, elem: Elem::B }),
            "h" => Some(Arrangement { count: 0, elem: Elem::H }),
            "s" => Some(Arrangement { count: 0, elem: Elem::S }),
            "d" => Some(Arrangement { count: 0, elem: Elem::D }),
            _ => None,
        })
    });
    let q = matches!(class, VClass::Q | VClass::Z) || arr.map(|a| a.count as u16 * a.elem.size() as u16 == 128).unwrap_or(false);
    Some((VReg { class, num, q, arr }, index))
}

pub fn parse_arrangement(s: &str) -> Option<Arrangement> {
    let s = s.trim().to_ascii_lowercase();
    let elem_pos = s.find(|c: char| c.is_ascii_alphabetic())?;
    let count: u8 = s[..elem_pos].parse().ok()?;
    let elem = match &s[elem_pos..] {
        "b" => Elem::B,
        "h" => Elem::H,
        "s" => Elem::S,
        "d" => Elem::D,
        "q" => Elem::Q,
        _ => return None,
    };
    Some(Arrangement { count, elem })
}

pub static REGISTER_NAMES: &[&str] = &[
    "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12",
    "x13", "x14", "x15", "x16", "x17", "x18", "x19", "x20", "x21", "x22", "x23", "x24",
    "x25", "x26", "x27", "x28", "x29", "x30", "sp", "xzr", "w0", "w1", "w2", "w3", "w4",
    "w5", "w6", "w7", "w8", "w9", "w10", "w11", "w12", "w13", "w14", "w15", "w16", "w17",
    "w18", "w19", "w20", "w21", "w22", "w23", "w24", "w25", "w26", "w27", "w28", "w29",
    "w30", "wsp", "wzr", "lr", "fp", "v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7",
    "q0", "q1", "q2", "q3", "q4", "q5", "q6", "q7", "d0", "d1", "s0", "s1", "h0", "h1",
    "b0", "b1", "z0", "z1", "p0", "p1",
];

pub static DIRECTIVES: &[&str] = &[
    ".text", ".data", ".rodata", ".bss", ".section", ".global", ".globl", ".type", ".size",
    ".align", ".balign", ".p2align", ".byte", ".hword", ".short", ".2byte", ".word", ".4byte",
    ".long", ".quad", ".8byte", ".ascii", ".asciz", ".string", ".space", ".skip", ".zero",
    ".equ", ".set", ".equiv", ".file", ".ident", ".arch", ".cpu", ".syntax", ".option",
    ".inst", ".ltorg", ".org", ".macro", ".endm", ".include", ".if", ".ifdef", ".ifndef",
    ".else", ".endif", ".func", ".endfunc", ".rept", ".endr", ".err", ".warning",
];

pub fn is_register_name(s: &str) -> bool {
    parse_gpr(s).is_some() || parse_vreg(s).is_some()
}
