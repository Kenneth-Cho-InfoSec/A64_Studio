//! AArch64 disassembly built on `yaxpeax-arm`.
//!
//! The decoder is pure Rust and works on every host, so the Machine Code and
//! Hex Patcher screens do not need an ARM CPU. Execution is the only feature
//! that requires real ARM64 hardware.

use serde::Serialize;
use yaxpeax_arch::{Arch, Decoder, U8Reader};
use yaxpeax_arm::armv8::a64::ARMv8;

#[derive(Clone, Debug, Serialize)]
pub struct DisasmInsn {
    /// Byte offset inside the supplied buffer.
    pub offset: u64,
    /// Absolute virtual address, when a base was supplied.
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
    pub mnemonic: String,
    pub operands: String,
    /// True when yaxpeax understood the word; false for `.word` data.
    pub valid: bool,
}

fn split_text(text: &str) -> (String, String) {
    let text = text.trim();
    match text.find(|c: char| c.is_whitespace()) {
        Some(i) => (
            text[..i].to_string(),
            text[i..].trim().to_string(),
        ),
        None => (text.to_string(), String::new()),
    }
}

/// Decode one 32-bit instruction. Returns `None` if fewer than four bytes are
/// available.
pub fn disassemble_one(bytes: &[u8], offset: u64, base: u64) -> Option<DisasmInsn> {
    if bytes.len() < 4 {
        return None;
    }
    let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let mut decoder = <ARMv8 as Arch>::Decoder::default();
    let mut reader = U8Reader::new(&bytes[..4]);
    let (text, valid) = match decoder.decode(&mut reader) {
        Ok(insn) => (format!("{}", insn), true),
        Err(_) => (format!(".word 0x{:08x}", word), false),
    };
    let (mnemonic, operands) = split_text(&text);
    Some(DisasmInsn {
        offset,
        address: base.wrapping_add(offset),
        bytes: bytes[..4].to_vec(),
        text,
        mnemonic,
        operands,
        valid,
    })
}

/// Decode a byte buffer as a sequence of 32-bit AArch64 instructions.
pub fn disassemble(code: &[u8], base: u64) -> Vec<DisasmInsn> {
    let mut out = Vec::with_capacity(code.len() / 4);
    let mut off = 0usize;
    while off + 4 <= code.len() {
        if let Some(insn) = disassemble_one(&code[off..off + 4], off as u64, base) {
            out.push(insn);
        }
        off += 4;
    }
    // Preserve trailing bytes as data words so the Machine Code view is exact.
    if off < code.len() {
        let tail = &code[off..];
        let mut word = [0u8; 4];
        word[..tail.len()].copy_from_slice(tail);
        let v = u32::from_le_bytes(word);
        out.push(DisasmInsn {
            offset: off as u64,
            address: base.wrapping_add(off as u64),
            bytes: tail.to_vec(),
            text: format!(".byte 0x{:02x}", tail[0]),
            mnemonic: ".byte".into(),
            operands: format!("0x{:02x}", tail[0]),
            valid: false,
        });
        let _ = v;
    }
    out
}

/// Format a hex dump with 16 bytes per line and an ASCII column.
pub fn hex_dump(bytes: &[u8], base: u64) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 + 32);
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let addr = base.wrapping_add((i * 16) as u64);
        out.push_str(&format!("{:016x}  ", addr));
        for j in 0..16 {
            if j < chunk.len() {
                out.push_str(&format!("{:02x} ", chunk[j]));
            } else {
                out.push_str("   ");
            }
            if j == 7 {
                out.push(' ');
            }
        }
        out.push_str(" |");
        for b in chunk {
            out.push(if *b >= 0x20 && *b < 0x7f { *b as char } else { '.' });
        }
        out.push_str("|\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_instructions() {
        let code = [0x40, 0x05, 0x80, 0xD2, 0xC0, 0x03, 0x5F, 0xD6];
        let d = disassemble(&code, 0x1000);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].mnemonic, "mov");
        assert!(d[0].text.contains("x0"));
        assert_eq!(d[1].mnemonic, "ret");
        assert_eq!(d[0].address, 0x1000);
        assert_eq!(d[1].address, 0x1004);
    }

    #[test]
    fn unknown_word_is_preserved() {
        let d = disassemble(&[0, 0, 0, 0], 0);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].bytes, vec![0, 0, 0, 0]);
    }
}
