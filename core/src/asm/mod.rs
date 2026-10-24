//! Two-pass GNU-style AArch64 assembler.
//!
//! Layout model: `.text` (plus its literal pool) starts at image offset 0,
//! `.data`/`.rodata`/`.bss` follow at a 16-byte boundary. All emitted code is
//! position independent within the image; only absolute data pointers produce
//! relocations, which the execution engine applies once the image has been
//! placed at its real address.

pub mod encode;
pub mod expr;

use crate::syntax::{is_known_mnemonic, parse_gpr};
use encode::Ctx;
use expr::{eval, SymbolResolver};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub message: String,
    pub severity: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub value: u64,
    pub section: String,
    pub global: bool,
    pub kind: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelocInfo {
    pub offset: u64,
    pub symbol: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InsnInfo {
    pub line: usize,
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub source: String,
    pub mnemonic: String,
    pub operands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssembleResult {
    pub ok: bool,
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub entry: u64,
    pub text_size: u64,
    pub data_size: u64,
    pub image_size: u64,
    pub symbols: Vec<SymbolInfo>,
    pub relocs: Vec<RelocInfo>,
    pub instructions: Vec<InsnInfo>,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
    pub line_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Text,
    Data,
}

impl Section {
    fn name(self) -> &'static str {
        match self { Section::Text => ".text", Section::Data => ".data" }
    }
}

#[derive(Clone, Debug)]
struct LabelDef {
    name: String,
    section: Section,
    offset: u64,
    global: bool,
    size: u64,
}

#[derive(Clone, Debug)]
struct NumLabel {
    number: u32,
    line: usize,
    order: usize,
    section: Section,
    offset: u64,
}

#[derive(Clone, Debug)]
enum StmtKind {
    Insn { mnemonic: String, operands: Vec<String>, mnem_col: usize, op_cols: Vec<usize> },
    Data { name: String, args: Vec<String>, col: usize },
    Align { bytes: u64, fill: u8 },
}

#[derive(Clone, Debug)]
struct Stmt {
    line: usize,
    section: Section,
    offset: u64,
    size: u64,
    kind: StmtKind,
    pool: Option<usize>,
    source: String,
}

#[derive(Clone, Debug)]
struct PoolEntry {
    target: String,
    col: usize,
    offset: u64,
}

struct Resolver<'a> {
    labels: &'a HashMap<String, u64>,
    equ: &'a HashMap<String, i64>,
    numeric: &'a [NumLabel],
    line: usize,
    order: usize,
    loc: i64,
    pool_symbols: &'a HashMap<String, u64>,
    data_base: u64,
}

impl<'a> SymbolResolver for Resolver<'a> {
    fn resolve(&self, name: &str) -> Option<i64> {
        if let Some(v) = self.equ.get(name) {
            return Some(*v);
        }
        if let Some(v) = self.pool_symbols.get(name) {
            return Some(*v as i64);
        }
        if let Some(v) = self.labels.get(name) {
            return Some(*v as i64);
        }
        if name.len() >= 2 && (name.ends_with('f') || name.ends_with('b')) {
            let (num, dir) = name.split_at(name.len() - 1);
            if let Ok(number) = num.parse::<u32>() {
                if dir == "f" {
                    let mut best: Option<&NumLabel> = None;
                    for n in self.numeric {
                        if n.number == number
                            && (n.line > self.line || (n.line == self.line && n.order > self.order))
                            && best.map(|b| (n.line, n.order) < (b.line, b.order)).unwrap_or(true)
                        {
                            best = Some(n);
                        }
                    }
                    return best.map(|n| section_value(n.section, n.offset, self.data_base) as i64);
                } else {
                    let mut best: Option<&NumLabel> = None;
                    for n in self.numeric {
                        if n.number == number
                            && (n.line < self.line || (n.line == self.line && n.order <= self.order))
                            && best.map(|b| (n.line, n.order) > (b.line, b.order)).unwrap_or(true)
                        {
                            best = Some(n);
                        }
                    }
                    return best.map(|n| section_value(n.section, n.offset, self.data_base) as i64);
                }
            }
        }
        None
    }
    fn location(&self) -> i64 {
        self.loc
    }
}

fn section_value(section: Section, offset: u64, data_base: u64) -> u64 {
    match section {
        Section::Text => offset,
        Section::Data => data_base + offset,
    }
}

/// Strip comments while preserving character columns. Tracks block comments
/// across lines. Strings are preserved verbatim.
fn strip_comment(line: &str, in_block: &mut bool) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    let mut in_str = false;
    let mut esc = false;
    while i < chars.len() {
        if *in_block {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                out.push(' ');
                out.push(' ');
                i += 2;
                *in_block = false;
                continue;
            }
            out.push(' ');
            i += 1;
            continue;
        }
        if in_str {
            out.push(chars[i]);
            if esc {
                esc = false;
            } else if chars[i] == '\\' {
                esc = true;
            } else if chars[i] == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if chars[i] == '"' {
            in_str = true;
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() {
                out.push(' ');
                i += 1;
            }
            break;
        }
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            *in_block = true;
            out.push(' ');
            out.push(' ');
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn col_u16(line: &str, char_col: usize) -> usize {
    line.chars().take(char_col).map(|c| c.len_utf16()).sum()
}

fn split_args_with_cols(s: &str) -> (Vec<String>, Vec<usize>) {
    let mut args = Vec::new();
    let mut cols = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut start: Option<usize> = None;
    let mut in_str = false;
    let mut esc = false;
    for (idx, c) in s.char_indices() {
        if in_str {
            cur.push(c);
            if esc { esc = false; }
            else if c == '\\' { esc = true; }
            else if c == '"' { in_str = false; }
            continue;
        }
        match c {
            '"' => { in_str = true; if start.is_none() { start = Some(idx); } cur.push(c); }
            '[' | '{' | '(' => { depth += 1; if start.is_none() { start = Some(idx); } cur.push(c); }
            ']' | '}' | ')' => { depth -= 1; if start.is_none() { start = Some(idx); } cur.push(c); }
            ',' if depth == 0 => {
                args.push(cur.trim().to_string());
                cols.push(start.unwrap_or(idx));
                cur.clear();
                start = None;
            }
            _ => {
                if !c.is_whitespace() && start.is_none() {
                    start = Some(idx);
                }
                cur.push(c);
            }
        }
    }
    if start.is_some() || !args.is_empty() {
        args.push(cur.trim().to_string());
        cols.push(start.unwrap_or(0));
    }
    (args, cols)
}

fn parse_label_at(s: &str) -> Option<(String, usize)> {
    let trimmed = s.trim_start();
    let lead = s.len() - trimmed.len();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    if bytes[0].is_ascii_digit() {
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
        if i < bytes.len() && bytes[i] == b':' {
            return Some((trimmed[..i].to_string(), lead + i + 1));
        }
        return None;
    }
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' {
            i += 1;
        } else {
            break;
        }
    }
    if i > 0 && i < bytes.len() && bytes[i] == b':' {
        // Do not treat `b.eq:` (not valid) or directives as labels.
        let name = &trimmed[..i];
        if name.starts_with('.') && crate::syntax::DIRECTIVES.contains(&name.to_ascii_lowercase().as_str()) {
            return None;
        }
        return Some((name.to_string(), lead + i + 1));
    }
    None
}

fn parse_quoted(s: &str, op: usize) -> Result<Vec<u8>, String> {
    let t = s.trim();
    if !(t.starts_with('"') && t.ends_with('"') && t.len() >= 2) {
        return Err(format!("expected a quoted string, found '{}'", t));
    }
    let inner = &t[1..t.len() - 1];
    let mut out = Vec::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        let e = chars.next().ok_or_else(|| format!("dangling escape in operand {}", op + 1))?;
        match e {
            'n' => out.push(b'\n'),
            't' => out.push(b'\t'),
            'r' => out.push(b'\r'),
            '0' => out.push(0),
            '\\' => out.push(b'\\'),
            '"' => out.push(b'"'),
            'x' => {
                let h1 = chars.next().and_then(|c| c.to_digit(16));
                let h2 = chars.next().and_then(|c| c.to_digit(16));
                match (h1, h2) {
                    (Some(a), Some(b)) => out.push((a * 16 + b) as u8),
                    _ => return Err("malformed \\x escape".into()),
                }
            }
            other => return Err(format!("unsupported escape '\\{}'", other)),
        }
    }
    Ok(out)
}

/// True if the expression references a relocatable symbol (single symbol plus
/// an optional constant addend).
fn relocatable_symbol(expr: &str, labels: &HashMap<String, u64>, equ: &HashMap<String, i64>) -> Option<String> {
    let t = expr.trim().trim_start_matches('#').trim();
    let t = t.strip_prefix(':').map(|s| s.split_once(':').map(|x| x.1).unwrap_or(s)).unwrap_or(t);
    let mut idents = Vec::new();
    let mut i = 0;
    let b = t.as_bytes();
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '$' {
            let start = i;
            i += 1;
            while i < b.len() {
                let d = b[i] as char;
                if d.is_ascii_alphanumeric() || d == '_' || d == '.' || d == '$' { i += 1; } else { break; }
            }
            let ident = &t[start..i];
            if !ident.chars().all(|c| c.is_ascii_digit()) {
                idents.push(ident.to_string());
            }
        } else {
            i += 1;
        }
    }
    if idents.len() == 1 && !equ.contains_key(&idents[0]) && labels.contains_key(&idents[0]) {
        Some(idents.remove(0))
    } else {
        None
    }
}

pub fn assemble(source: &str) -> AssembleResult {
    let lines: Vec<&str> = source.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut stmts: Vec<Stmt> = Vec::new();
    let mut labels: Vec<LabelDef> = Vec::new();
    let mut numeric: Vec<NumLabel> = Vec::new();
    let mut globals: Vec<String> = Vec::new();
    let mut equ: HashMap<String, i64> = HashMap::new();
    let mut section = Section::Text;
    let mut text_off = 0u64;
    let mut data_off = 0u64;
    let mut in_block = false;
    let mut pool: Vec<PoolEntry> = Vec::new();
    let mut pending_labels: Vec<(String, usize)> = Vec::new();

    let mut err = |line: usize, col: usize, len: usize, msg: String, code: &str| {
        errors.push(Diagnostic { line, col, len, message: msg, severity: "error".into(), code: code.into() });
    };

    // ---------- pass 1: parse, size, labels, directives ----------
    for (idx, raw) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let stripped = strip_comment(raw, &mut in_block);
        let mut rest = stripped.as_str();
        let mut consumed = 0usize;
        pending_labels.clear();

        while let Some((name, end)) = parse_label_at(rest) {
            let is_num = name.chars().all(|c| c.is_ascii_digit());
            if is_num {
                numeric.push(NumLabel {
                    number: name.parse().unwrap_or(0),
                    line: line_no,
                    order: numeric.len(),
                    section,
                    offset: if section == Section::Text { text_off } else { data_off },
                });
            } else {
                let off = if section == Section::Text { text_off } else { data_off };
                if labels.iter().any(|l| l.name == name && l.section == section && l.offset == off) {
                    err(line_no, col_u16(raw, consumed), name.len(), format!("label '{}' is defined more than once", name), "asm.duplicate-label");
                }
                labels.push(LabelDef { name: name.clone(), section, offset: off, global: false, size: 0 });
                pending_labels.push((name, line_no));
            }
            consumed += end;
            rest = &rest[end..];
        }

        let body = rest.trim_start();
        let body_lead = rest.len() - body.len();
        let body_col = consumed + body_lead;
        if body.is_empty() {
            continue;
        }
        if body.starts_with('#') {
            let t = body.trim_start_matches('#').trim_start();
            if t.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                err(line_no, col_u16(raw, body_col), body.len(),
                    "C preprocessor directives are not available in A64 Studio. Save the file as .s (not .S) and remove #include/#define lines.".into(),
                    "asm.preprocessor");
            } else {
                err(line_no, col_u16(raw, body_col), body.len(), "unexpected '#' at the start of a statement".into(), "asm.syntax");
            }
            continue;
        }

        if body.starts_with('.') {
            let (name, args_str) = match body.find(char::is_whitespace) {
                Some(p) => (&body[..p], body[p..].trim()),
                None => (body, ""),
            };
            let dname = name.to_ascii_lowercase();
            let (args, cols) = split_args_with_cols(args_str);
            let args_col = if args_str.is_empty() { body_col } else { body_col + body.find(args_str).unwrap_or(0) };
            let _ = cols;
            let off = if section == Section::Text { text_off } else { data_off };
            match dname.as_str() {
                ".text" => { section = Section::Text; }
                ".data" | ".rodata" | ".bss" => { section = Section::Data; }
                ".section" => {
                    let target = args.first().map(|s| s.trim().trim_matches('"')).unwrap_or("");
                    if target.starts_with(".text") {
                        section = Section::Text;
                    } else if target.starts_with(".data") || target.starts_with(".rodata") || target.starts_with(".bss") {
                        section = Section::Data;
                    } else {
                        err(line_no, col_u16(raw, args_col), name.len(),
                            format!("section '{}' is not supported; use .text, .data, .rodata or .bss", target),
                            "asm.section");
                    }
                }
                ".global" | ".globl" => {
                    for a in &args {
                        let n = a.trim();
                        if !n.is_empty() {
                            globals.push(n.to_string());
                        }
                    }
                }
                ".type" | ".size" | ".file" | ".ident" | ".arch" | ".cpu" | ".syntax" | ".option" | ".ltorg" | ".func" | ".endfunc" => {
                    if dname == ".size" {
                        if let Some(n) = args.first() {
                            let n = n.trim();
                            if let Some(l) = labels.iter_mut().find(|l| l.name == n && l.section == section) {
                                if let Some(expr) = args.get(1) {
                                    let normalized: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
                                    if normalized == ".-" || normalized == "." || normalized == format!(".-{}", n) {
                                        l.size = off.saturating_sub(l.offset);
                                    }
                                }
                            }
                        }
                    }
                }
                ".equ" | ".set" | ".equiv" => {
                    if args.len() < 2 {
                        err(line_no, col_u16(raw, body_col), body.len(), format!("'{}' requires a name and an expression", dname), "asm.directive");
                    } else {
                        let n = args[0].trim().to_string();
                        let e = args[1..].join(",");
                        let loc = off as i64;
                        let empty_labels = HashMap::new();
                        let empty_numeric: Vec<NumLabel> = Vec::new();
                        let empty_pool = HashMap::new();
                        let r = Resolver { labels: &empty_labels, equ: &equ, numeric: &empty_numeric, line: line_no, order: 0, loc, pool_symbols: &empty_pool, data_base: 0 };
                        match eval(&e, &r) {
                            Ok(v) => {
                                if let std::collections::hash_map::Entry::Vacant(entry) = equ.entry(n) {
                                    entry.insert(v);
                                } else {
                                    err(line_no, col_u16(raw, body_col), name.len(), format!("symbol '{}' is already defined", args[0].trim()), "asm.duplicate-symbol");
                                }
                            }
                            Err(e2) => err(line_no, col_u16(raw, body_col) + e2.col, 1,
                                format!("in .equ: {} (labels are not allowed in .equ in this assembler)", e2.message), "asm.expr"),
                        }
                    }
                }
                ".align" | ".balign" | ".p2align" => {
                    let n = args.first().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
                    let align_bytes = match dname.as_str() {
                        ".align" | ".p2align" => {
                            if n > 20 { err(line_no, col_u16(raw, body_col), name.len(), "alignment is too large".into(), "asm.directive"); 1 } else { 1u64 << n }
                        }
                        _ => n.max(1),
                    };
                    let fill = args.get(1).and_then(|s| s.trim().trim_start_matches('#').parse::<u8>().ok()).unwrap_or(0);
                    let cur = if section == Section::Text { text_off } else { data_off };
                    let pad = (align_bytes - (cur % align_bytes)) % align_bytes;
                    stmts.push(Stmt { line: line_no, section, offset: cur, size: pad, kind: StmtKind::Align { bytes: align_bytes, fill }, pool: None, source: raw.trim().to_string() });
                    if section == Section::Text { text_off += pad; } else { data_off += pad; }
                }
                ".byte" | ".hword" | ".short" | ".2byte" | ".word" | ".4byte" | ".long" | ".quad" | ".8byte" => {
                    let width = match dname.as_str() {
                        ".byte" => 1,
                        ".hword" | ".short" | ".2byte" => 2,
                        ".word" | ".4byte" | ".long" => 4,
                        _ => 8,
                    };
                    let size = width * args.len() as u64;
                    stmts.push(Stmt { line: line_no, section, offset: off, size, kind: StmtKind::Data { name: dname.clone(), args, col: body_col }, pool: None, source: raw.trim().to_string() });
                    if section == Section::Text { text_off += size; } else { data_off += size; }
                }
                ".ascii" | ".asciz" | ".string" => {
                    let mut size = 0u64;
                    for a in &args {
                        match parse_quoted(a, 0) {
                            Ok(b) => size += b.len() as u64 + if dname == ".ascii" { 0 } else { 1 },
                            Err(e) => err(line_no, col_u16(raw, body_col), name.len(), e, "asm.string"),
                        }
                    }
                    stmts.push(Stmt { line: line_no, section, offset: off, size, kind: StmtKind::Data { name: dname.clone(), args, col: body_col }, pool: None, source: raw.trim().to_string() });
                    if section == Section::Text { text_off += size; } else { data_off += size; }
                }
                ".space" | ".skip" | ".zero" => {
                    let n = args.first().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
                    if n > 1 << 20 {
                        err(line_no, col_u16(raw, body_col), name.len(), "reserved space is limited to 1 MiB".into(), "asm.limit");
                    }
                    let size = n.min(1 << 20);
                    stmts.push(Stmt { line: line_no, section, offset: off, size, kind: StmtKind::Data { name: dname.clone(), args, col: body_col }, pool: None, source: raw.trim().to_string() });
                    if section == Section::Text { text_off += size; } else { data_off += size; }
                }
                ".inst" => {
                    let size = 4 * args.len() as u64;
                    stmts.push(Stmt { line: line_no, section, offset: off, size, kind: StmtKind::Data { name: dname.clone(), args, col: body_col }, pool: None, source: raw.trim().to_string() });
                    if section == Section::Text { text_off += size; } else { data_off += size; }
                }
                ".macro" | ".endm" | ".rept" | ".endr" | ".if" | ".ifdef" | ".ifndef" | ".else" | ".endif" | ".include" | ".org" => {
                    err(line_no, col_u16(raw, body_col), name.len(),
                        format!("'{}' is not supported: A64 Studio has no macro processor, conditional assembly or file inclusion. Expand the source or use a separate preprocessor.", dname),
                        "asm.unsupported-directive");
                }
                ".err" => {
                    err(line_no, col_u16(raw, body_col), name.len(), args.join(" "), "asm.user-error");
                }
                ".warning" => {
                    warnings.push(Diagnostic { line: line_no, col: col_u16(raw, body_col), len: name.len(), message: args.join(" "), severity: "warning".into(), code: "asm.user-warning".into() });
                }
                _ => {
                    err(line_no, col_u16(raw, body_col), name.len(), format!("unknown assembler directive '{}'", name), "asm.unknown-directive");
                }
            }
            continue;
        }

        // Instruction.
        let (mnemonic, ops_str) = match body.find(char::is_whitespace) {
            Some(p) => (&body[..p], body[p..].trim()),
            None => (body, ""),
        };
        let mnem_col = body_col;
        let lower = mnemonic.to_ascii_lowercase();
        let ops_off = if ops_str.is_empty() { 0 } else { body.find(ops_str).unwrap_or(mnemonic.len()) };
        let (mut operands, mut op_cols) = split_args_with_cols(ops_str);
        // Merge shift suffixes into the previous operand (columns follow the
        // first part of the merged operand).
        {
            let mut merged = Vec::new();
            let mut merged_cols = Vec::new();
            for (i, op) in operands.iter().enumerate() {
                let first = op.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
                let suffix = matches!(first.as_str(), "lsl" | "lsr" | "asr" | "ror" | "uxtb" | "uxth" | "uxtw" | "uxtx" | "sxtb" | "sxth" | "sxtw" | "sxtx");
                if suffix && !merged.is_empty() {
                    let last: &mut String = merged.last_mut().unwrap();
                    last.push(' ');
                    last.push_str(op);
                } else {
                    merged.push(op.clone());
                    merged_cols.push(op_cols[i]);
                }
            }
            // A post-index offset is a separate top-level operand:
            // [x0], #16  ->  merge into the memory operand.
            let mut merged2 = Vec::new();
            let mut merged2_cols = Vec::new();
            for (i, op) in merged.iter().enumerate() {
                let t = op.trim();
                if t.starts_with('#')
                    && merged2.last().map(|p: &String| p.trim_end().ends_with(']')).unwrap_or(false)
                {
                    let last: &mut String = merged2.last_mut().unwrap();
                    last.push_str(", ");
                    last.push_str(t);
                } else {
                    merged2.push(op.clone());
                    merged2_cols.push(merged_cols[i]);
                }
            }
            operands = merged2;
            op_cols = merged2_cols;
        }
        for c in &mut op_cols {
            *c += body_col + ops_off;
        }

        let cond_branch = lower.starts_with("b.") && crate::syntax::condition_code(&lower[2..]).is_some();
        if !is_known_mnemonic(&lower) && !cond_branch {
            err(line_no, col_u16(raw, mnem_col), mnemonic.len(),
                format!("unknown or unsupported instruction '{}'", mnemonic),
                "asm.unknown-mnemonic");
            // still advance by 4 to keep subsequent offsets sane
            if section == Section::Text { text_off += 4; }
            continue;
        }

        let off = if section == Section::Text { text_off } else { data_off };
        let mut pool_id = None;
        let is_ldr_pool = matches!(lower.as_str(), "ldr" | "ldrsw" | "prfm")
            && operands.len() == 2
            && operands[1].trim().starts_with('=');
        if is_ldr_pool {
            let target = operands[1].trim().trim_start_matches('=').trim().to_string();
            if target.is_empty() {
                err(line_no, col_u16(raw, mnem_col), mnemonic.len(), "empty literal after '='".into(), "asm.syntax");
            } else {
                pool_id = Some(pool.len());
                pool.push(PoolEntry { target, col: op_cols.get(1).copied().unwrap_or(0), offset: 0 });
                operands[1] = String::new(); // filled in pass 2
            }
        } else if lower == "mov" && operands.len() == 2 && operands[1].trim().starts_with('#') {
            // Large/symbolic moves become literal loads; this keeps every
            // statement exactly one instruction long.
            let rd = parse_gpr(operands[0].trim());
            let target = operands[1].trim().trim_start_matches('#').trim().to_string();
            let mut need_pool = false;
            if let Some(rd) = rd {
                let empty_labels = HashMap::new();
                let empty_numeric: Vec<NumLabel> = Vec::new();
                let empty_pool = HashMap::new();
                let r = Resolver { labels: &empty_labels, equ: &equ, numeric: &empty_numeric, line: line_no, order: 0, loc: off as i64, pool_symbols: &empty_pool, data_base: 0 };
                match eval(&target, &r) {
                    Ok(v) => {
                        if !encode::mov_imm_encodable(v, rd.width) {
                            need_pool = true;
                        }
                    }
                    Err(_) => need_pool = true,
                }
                if rd.is_sp {
                    need_pool = false;
                }
            }
            if need_pool {
                pool_id = Some(pool.len());
                pool.push(PoolEntry { target, col: op_cols.get(1).copied().unwrap_or(0), offset: 0 });
            }
        }
        stmts.push(Stmt {
            line: line_no,
            section,
            offset: off,
            size: 4,
            kind: StmtKind::Insn { mnemonic: lower, operands, mnem_col, op_cols },
            pool: pool_id,
            source: raw.trim().to_string(),
        });
        if section == Section::Text { text_off += 4; } else { data_off += 4; }
    }

    // Apply global attributes.
    for g in &globals {
        let mut found = false;
        for l in labels.iter_mut() {
            if &l.name == g {
                l.global = true;
                found = true;
            }
        }
        if !found && !equ.contains_key(g) {
            err(1, 0, g.len(), format!("symbol '{}' is declared global but never defined", g), "asm.undefined");
        }
    }

    // Literal pool at the end of .text, 8-byte aligned.
    let pool_base = (text_off + 7) & !7;
    for (i, p) in pool.iter_mut().enumerate() {
        p.offset = pool_base + (i as u64) * 8;
    }
    let text_size = pool_base + pool.len() as u64 * 8;
    let data_size = data_off;
    // .data starts on a page boundary so the execution engine can map code
    // pages RX and data pages RW independently (Android W^X policy).
    let data_base = if data_size > 0 { (text_size + 4095) & !4095 } else { text_size };
    let image_size = data_base + data_size;

    // Symbol table values (image offsets).
    let mut label_values: HashMap<String, u64> = HashMap::new();
    for l in &labels {
        label_values.insert(l.name.clone(), section_value(l.section, l.offset, data_base));
    }
    let mut pool_symbols: HashMap<String, u64> = HashMap::new();
    for (i, p) in pool.iter().enumerate() {
        pool_symbols.insert(format!(".L__pool{}", i), p.offset);
    }

    // ---------- pass 2: encode ----------
    let mut text_bytes = vec![0u8; text_size as usize];
    let mut data_bytes = vec![0u8; data_size as usize];
    let mut relocs: Vec<RelocInfo> = Vec::new();
    let mut instructions: Vec<InsnInfo> = Vec::new();
    let mut pool_values: Vec<u64> = vec![0; pool.len()];

    for stmt in &stmts {
        let sec_base = match stmt.section { Section::Text => 0, Section::Data => data_base };
        let loc = sec_base + stmt.offset;
        let r = Resolver {
            labels: &label_values,
            equ: &equ,
            numeric: &numeric,
            line: stmt.line,
            order: 0,
            loc: loc as i64,
            pool_symbols: &pool_symbols,
            data_base,
        };
        match &stmt.kind {
            StmtKind::Insn { mnemonic, operands, mnem_col, op_cols } => {
                if stmt.section != Section::Text {
                    err(stmt.line, *mnem_col, mnemonic.len(), "instructions are only allowed in .text".into(), "asm.section");
                    continue;
                }
                let mut ops = operands.clone();
                let mut mnem = mnemonic.clone();
                if let Some(pid) = stmt.pool {
                    ops[1] = format!(".L__pool{}", pid);
                    mnem = if mnemonic == "mov" { "ldr".to_string() } else { mnemonic.clone() };
                }
                let ctx = Ctx { pc: loc, eval: &|s: &str| eval(s, &r).map_err(|e| e.message), pool_offset: stmt.pool.map(|p| pool[p].offset), section_base: sec_base };
                match encode::encode(&mnem, &ops, &ctx) {
                    Ok(words) => {
                        let mut bytes = Vec::with_capacity(words.len() * 4);
                        for w in &words {
                            bytes.extend_from_slice(&w.to_le_bytes());
                        }
                        let at = stmt.offset as usize;
                        if at + bytes.len() <= text_bytes.len() {
                            text_bytes[at..at + bytes.len()].copy_from_slice(&bytes);
                        }
                        instructions.push(InsnInfo {
                            line: stmt.line,
                            offset: stmt.offset,
                            bytes,
                            source: stmt.source.clone(),
                            mnemonic: mnem.clone(),
                            operands: ops.clone(),
                        });
                        // Fill literal pool entry.
                        if let Some(pid) = stmt.pool {
                            let target = &pool[pid].target;
                            let value = match eval(target, &r) {
                                Ok(v) => v as u64,
                                Err(e) => {
                                    err(stmt.line, pool[pid].col, target.len(), format!("literal expression: {}", e.message), "asm.expr");
                                    0
                                }
                            };
                            pool_values[pid] = value;
                            if let Some(sym) = relocatable_symbol(target, &label_values, &equ) {
                                relocs.push(RelocInfo { offset: pool[pid].offset, symbol: sym, kind: "abs64".into() });
                            }
                        }
                    }
                    Err(e) => {
                        let (col, len) = match e.operand {
                            Some(i) if i < ops.len() => {
                                let c = op_cols.get(i).copied().unwrap_or(*mnem_col);
                                let l = ops[i].chars().count().max(1);
                                (c, l)
                            }
                            _ => (*mnem_col, mnemonic.len()),
                        };
                        let code = if e.message.contains("not supported") || e.message.contains("not implemented") {
                            "asm.unsupported"
                        } else if e.message.contains("range") || e.message.contains("fit") || e.message.contains("out of") {
                            "asm.range"
                        } else {
                            "asm.encoding"
                        };
                        err(stmt.line, col_u16(lines[stmt.line - 1], col), len, e.message, code);
                    }
                }
            }
            StmtKind::Data { name, args, col } => {
                let width = match name.as_str() {
                    ".byte" => 1,
                    ".hword" | ".short" | ".2byte" => 2,
                    ".word" | ".4byte" | ".long" => 4,
                    ".quad" | ".8byte" => 8,
                    _ => 0,
                };
                let dst: &mut Vec<u8> = if stmt.section == Section::Text { &mut text_bytes } else { &mut data_bytes };
                let mut pos = stmt.offset as usize;
                match name.as_str() {
                    ".byte" | ".hword" | ".short" | ".2byte" | ".word" | ".4byte" | ".long" | ".quad" | ".8byte" => {
                        for a in args {
                            match eval(a, &r) {
                                Ok(v) => {
                                    let mut buf = [0u8; 8];
                                    buf[..8].copy_from_slice(&(v as u64).to_le_bytes());
                                    if pos + width as usize <= dst.len() {
                                        dst[pos..pos + width as usize].copy_from_slice(&buf[..width as usize]);
                                    }
                                    if width == 8 {
                                        if let Some(sym) = relocatable_symbol(a, &label_values, &equ) {
                                            relocs.push(RelocInfo { offset: loc + (pos as u64 - stmt.offset), symbol: sym, kind: "abs64".into() });
                                        }
                                    }
                                }
                                Err(e) => err(stmt.line, col_u16(lines[stmt.line - 1], *col) + e.col, 1, e.message, "asm.expr"),
                            }
                            pos += width as usize;
                        }
                    }
                    ".ascii" | ".asciz" | ".string" => {
                        for a in args {
                            match parse_quoted(a, 0) {
                                Ok(b) => {
                                    if pos + b.len() <= dst.len() {
                                        dst[pos..pos + b.len()].copy_from_slice(&b);
                                    }
                                    pos += b.len();
                                    if name != ".ascii" {
                                        if pos < dst.len() { dst[pos] = 0; }
                                        pos += 1;
                                    }
                                }
                                Err(e) => err(stmt.line, col_u16(lines[stmt.line - 1], *col), 1, e, "asm.string"),
                            }
                        }
                    }
                    ".space" | ".skip" | ".zero" => {
                        let fill = args.get(1).and_then(|s| s.trim().trim_start_matches('#').parse::<u8>().ok()).unwrap_or(0);
                        let n = args.first().and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(0).min(1 << 20);
                        for i in 0..n {
                            if pos + i < dst.len() { dst[pos + i] = fill; }
                        }
                    }
                    ".inst" => {
                        for a in args {
                            match eval(a, &r) {
                                Ok(v) => {
                                    let b = (v as u32).to_le_bytes();
                                    if pos + 4 <= dst.len() { dst[pos..pos + 4].copy_from_slice(&b); }
                                }
                                Err(e) => err(stmt.line, col_u16(lines[stmt.line - 1], *col) + e.col, 1, e.message, "asm.expr"),
                            }
                            pos += 4;
                        }
                    }
                    _ => {}
                }
            }
            StmtKind::Align { bytes, fill } => {
                let start = if stmt.section == Section::Text { &mut text_bytes } else { &mut data_bytes };
                let at = stmt.offset as usize;
                for i in at..(at + stmt.size as usize).min(start.len()) {
                    start[i] = *fill;
                }
                let _ = bytes;
            }
        }
    }

    // Write pool values.
    for (i, v) in pool_values.iter().enumerate() {
        let at = pool[i].offset as usize;
        if at + 8 <= text_bytes.len() {
            text_bytes[at..at + 8].copy_from_slice(&v.to_le_bytes());
        }
    }

    // Entry point.
    let entry = ["asm_main", "main", "_start"]
        .iter()
        .find_map(|n| label_values.get(*n).copied())
        .or_else(|| {
            labels.iter().find(|l| l.section == Section::Text).map(|l| l.offset)
        })
        .unwrap_or(0);

    let mut symbols: Vec<SymbolInfo> = labels
        .iter()
        .map(|l| SymbolInfo {
            name: l.name.clone(),
            value: section_value(l.section, l.offset, data_base),
            section: l.section.name().to_string(),
            global: l.global,
            kind: "label".into(),
            size: l.size,
        })
        .collect();
    for (n, v) in &equ {
        symbols.push(SymbolInfo { name: n.clone(), value: *v as u64, section: "ABS".into(), global: false, kind: "equ".into(), size: 0 });
    }
    symbols.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));

    AssembleResult {
        ok: errors.is_empty(),
        code: text_bytes,
        data: data_bytes,
        entry,
        text_size,
        data_size,
        image_size,
        symbols,
        relocs,
        instructions,
        errors,
        warnings,
        line_count: lines.len(),
    }
}

/// Apply relocations to a loaded image.
pub fn apply_relocs(image: &mut [u8], relocs: &[RelocInfo], base: u64, labels: &HashMap<String, u64>) {
    for r in relocs {
        if let Some(v) = labels.get(&r.symbol) {
            let off = r.offset as usize;
            if off + 8 <= image.len() {
                let val = base.wrapping_add(*v);
                image[off..off + 8].copy_from_slice(&val.to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_function() {
        let r = assemble(".text\n.global asm_main\nasm_main:\n    mov x0, #42\n    ret\n");
        assert!(r.ok, "{:?}", r.errors);
        assert_eq!(r.code[..8], [0x40, 0x05, 0x80, 0xD2, 0xC0, 0x03, 0x5F, 0xD6]);
        assert_eq!(r.entry, 0);
        assert_eq!(r.instructions.len(), 2);
    }

    #[test]
    fn labels_and_branches() {
        let r = assemble("start:\n  mov x0, #0\nloop:\n  add x0, x0, #1\n  cmp x0, #10\n  b.ne loop\n  ret\n");
        assert!(r.ok, "{:?}", r.errors);
        assert_eq!(r.instructions[3].bytes, [0xC1, 0xFF, 0xFF, 0x54]);
    }

    #[test]
    fn literal_pool() {
        let r = assemble("mov x0, #0x123456789abcdef0\nret\n");
        assert!(r.ok, "{:?}", r.errors);
        assert_eq!(r.instructions[0].mnemonic, "ldr");
        assert_eq!(r.text_size, 16);
    }

    #[test]
    fn data_and_strings() {
        let r = assemble(".data\nmsg: .asciz \"hi\"\n.text\nldr x0, msg\nret\n");
        assert!(r.ok, "{:?}", r.errors);
        assert_eq!(&r.data[..3], b"hi\0");
        assert_eq!(r.symbols.iter().find(|s| s.name == "msg").unwrap().value, (r.text_size + 4095) & !4095);
    }

    #[test]
    fn size_directive_records_label_extent() {
        let r = assemble(".text\nfn:\n    mov x0, #7\n    ret\n.size fn, .-fn\n");
        assert!(r.ok, "{:?}", r.errors);
        assert_eq!(r.symbols.iter().find(|s| s.name == "fn").unwrap().size, 8);
    }

    #[test]
    fn errors_have_positions() {
        let r = assemble("mov x0, #1\nfrobnicate x0\nadd x0, x1, #0x1000000\n");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.line == 2 && e.code == "asm.unknown-mnemonic"));
        assert!(r.errors.iter().any(|e| e.line == 3 && e.code == "asm.range"));
    }

    #[test]
    fn unsupported_directive() {
        let r = assemble(".macro foo\n.endm\n");
        assert!(!r.ok);
        assert!(r.errors.iter().any(|e| e.code == "asm.unsupported-directive"));
    }
}
