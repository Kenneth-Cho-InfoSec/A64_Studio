//! Incremental AArch64 syntax lexer.
//!
//! The lexer is line oriented so that a keystroke only re-tokenises the lines
//! that changed (plus any following lines whose block-comment state changed).
//! Offsets are UTF-16 code units, matching Kotlin/Compose string indices.

use crate::syntax::*;

#[derive(Clone, Debug)]
pub struct Token {
    pub start: u32,
    pub end: u32,
    pub kind: TokenKind,
}

#[derive(Clone, Debug)]
struct Line {
    text: String,
    tokens: Vec<Token>,
    start_in_block: bool,
    end_in_block: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct LexState {
    in_block_comment: bool,
}

pub struct HighlightSession {
    lines: Vec<Line>,
    text: String,
    pub revision: u64,
}

impl Default for HighlightSession {
    fn default() -> Self {
        Self::new()
    }
}

impl HighlightSession {
    pub fn new() -> Self {
        Self { lines: Vec::new(), text: String::new(), revision: 0 }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replace the document text. Returns the inclusive line range that was
    /// re-tokenised so callers can refresh only that part if they wish.
    pub fn set_text(&mut self, text: &str) -> (usize, usize) {
        self.revision += 1;
        let new_lines: Vec<&str> = split_lines(text);
        let old_len = self.lines.len();
        let new_len = new_lines.len();

        // Common prefix.
        let mut prefix = 0;
        while prefix < old_len && prefix < new_len && self.lines[prefix].text == new_lines[prefix] {
            prefix += 1;
        }
        // Common suffix (not overlapping the prefix).
        let mut suffix = 0;
        while suffix < old_len.saturating_sub(prefix)
            && suffix < new_len.saturating_sub(prefix)
            && self.lines[old_len - 1 - suffix].text == new_lines[new_len - 1 - suffix]
        {
            suffix += 1;
        }

        let new_changed_end = new_len - suffix; // exclusive

        // If block-comment state at the first changed line differs from what
        // the cached tokens assumed, extend the relex range backwards to the
        // beginning of the document (block comments are rare; correctness
        // first).
        let mut start = prefix.min(old_len);
        let cached_state = if start < old_len {
            self.lines[start].start_in_block
        } else {
            false
        };
        if cached_state {
            start = 0;
        }

        let state_before = if start == 0 {
            LexState::default()
        } else {
            LexState { in_block_comment: self.lines[start - 1].end_in_block }
        };

        // Re-tokenise changed region, then keep going while the block comment
        // state does not match the cached state of the unchanged tail.
        let mut new_cache: Vec<Line> = Vec::with_capacity(new_len);
        new_cache.extend_from_slice(&self.lines[..start]);
        let mut state = state_before;
        let mut i = start;
        let mut stop_at = new_changed_end;
        loop {
            if i >= new_len {
                break;
            }
            let l = lex_line(new_lines[i], state);
            state = LexState { in_block_comment: l.end_in_block };
            new_cache.push(l);
            i += 1;
            if i >= stop_at {
                if i < new_len && i < old_len - suffix {
                    // still in changed region
                    continue;
                }
                // If unchanged tail exists, verify its cached start state.
                if suffix > 0 {
                    let tail_idx = new_len - suffix;
                    if new_cache.last().map(|l| l.end_in_block) == Some(self.lines[tail_idx].start_in_block) {
                        new_cache.extend_from_slice(&self.lines[tail_idx..]);
                        break;
                    } else {
                        // state changed: relex the rest
                        stop_at = new_len;
                        continue;
                    }
                } else {
                    break;
                }
            }
        }
        // Defensive: ensure all lines present.
        while new_cache.len() < new_len {
            let idx = new_cache.len();
            let l = lex_line(new_lines[idx], LexState { in_block_comment: new_cache.last().map(|x| x.end_in_block).unwrap_or(false) });
            new_cache.push(l);
        }

        self.lines = new_cache;
        self.text.clear();
        self.text.push_str(text);
        let end = new_changed_end.max(start + 1).min(new_len.max(1));
        (start, end.saturating_sub(1))
    }

    pub fn token_string(&self, from_line: usize, to_line: usize) -> String {
        let mut out = String::with_capacity(self.text.len() / 3 + 64);
        let from = from_line.saturating_sub(1).min(self.lines.len());
        let to = to_line.min(self.lines.len());
        for (idx, line) in self.lines.iter().enumerate().skip(from).take(to.saturating_sub(from)) {
            for t in &line.tokens {
                out.push_str(&(idx + 1).to_string());
                out.push(':');
                out.push_str(&t.start.to_string());
                out.push(':');
                out.push_str(&t.end.to_string());
                out.push(':');
                out.push_str(&t.kind.code().to_string());
                out.push('\n');
            }
        }
        out
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_text(&self, line: usize) -> Option<&str> {
        self.lines.get(line.saturating_sub(1)).map(|l| l.text.as_str())
    }

    /// Label declarations and references, used for completion and matching.
    pub fn labels(&self) -> Vec<(String, usize, bool)> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (idx, line) in self.lines.iter().enumerate() {
            for t in &line.tokens {
                if t.kind == TokenKind::LabelDecl {
                    let text = slice_u16(&line.text, t.start, t.end);
                    if seen.insert(text.clone()) {
                        out.push((text, idx + 1, true));
                    }
                }
            }
        }
        out
    }
}

fn split_lines(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = text.split('\n').collect();
    // A trailing newline means an empty final line; split already yields it.
    if out.is_empty() {
        out.push("");
    }
    for l in out.iter_mut() {
        if l.ends_with('\r') {
            *l = &l[..l.len() - 1];
        }
    }
    out
}

pub fn slice_u16(s: &str, start: u32, end: u32) -> String {
    let mut out = String::new();
    let mut u = 0u32;
    for ch in s.chars() {
        let w = ch.len_utf16() as u32;
        if u >= start && u + w <= end {
            out.push(ch);
        }
        u += w;
    }
    out
}

fn lex_line(text: &str, state: LexState) -> Line {
    let chars: Vec<char> = text.chars().collect();
    let mut u16_at = Vec::with_capacity(chars.len() + 1);
    let mut u = 0u32;
    for c in &chars {
        u16_at.push(u);
        u += c.len_utf16() as u32;
    }
    u16_at.push(u);

    let mut tokens = Vec::new();
    let mut i = 0usize;
    let mut in_block = state.in_block_comment;
    let mut expect_insn = true;
    let mut after_comma = false;

    let push = |tokens: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind| {
        if end > start {
            tokens.push(Token { start: u16_at[start], end: u16_at[end], kind });
        }
    };

    if in_block {
        if let Some(pos) = find_sub(&chars, 0, &['*', '/']) {
            push(&mut tokens, 0, pos + 2, TokenKind::Comment);
            i = pos + 2;
            in_block = false;
        } else {
            push(&mut tokens, 0, chars.len(), TokenKind::Comment);
            return Line { text: text.to_string(), tokens, start_in_block: true, end_in_block: true };
        }
    }

    while i < chars.len() {
        let c = chars[i];
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            push(&mut tokens, i, chars.len(), TokenKind::Comment);
            break;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(pos) = find_sub(&chars, i + 2, &['*', '/']) {
                push(&mut tokens, i, pos + 2, TokenKind::Comment);
                i = pos + 2;
                continue;
            } else {
                push(&mut tokens, i, chars.len(), TokenKind::Comment);
                in_block = true;
                break;
            }
        }
        if c == '"' {
            let mut j = i + 1;
            let mut closed = false;
            while j < chars.len() {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    j += 1;
                    closed = true;
                    break;
                }
                j += 1;
            }
            if closed {
                push(&mut tokens, i, j, TokenKind::String);
            } else {
                push(&mut tokens, i, chars.len(), TokenKind::Invalid);
            }
            i = j.min(chars.len());
            expect_insn = false;
            continue;
        }
        if c == '#' {
            let mut j = i + 1;
            if j < chars.len() && chars[j] == '(' {
                let mut depth = 1;
                j += 1;
                while j < chars.len() && depth > 0 {
                    if chars[j] == '(' {
                        depth += 1;
                    } else if chars[j] == ')' {
                        depth -= 1;
                    }
                    j += 1;
                }
            } else {
                while j < chars.len() {
                    let d = chars[j];
                    if d.is_whitespace() || d == ',' || d == ']' || d == '}' || d == ')' || d == '"' {
                        break;
                    }
                    if d == '/' && j + 1 < chars.len() && (chars[j + 1] == '/' || chars[j + 1] == '*') {
                        break;
                    }
                    j += 1;
                }
            }
            push(&mut tokens, i, j, TokenKind::Immediate);
            i = j;
            after_comma = false;
            continue;
        }
        // Local numeric label `1:` at statement start, or `1f` / `1b` refs.
        if c.is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len() && chars[j] == ':' && expect_insn {
                push(&mut tokens, i, j + 1, TokenKind::LabelDecl);
                i = j + 1;
                expect_insn = true;
                continue;
            }
            if j < chars.len() && (chars[j] == 'f' || chars[j] == 'b')
                && (j + 1 >= chars.len() || !is_ident_continue(chars[j + 1]))
            {
                push(&mut tokens, i, j + 1, TokenKind::LabelRef);
                i = j + 1;
                after_comma = false;
                continue;
            }
            // Number literal.
            let mut k = j;
            if k < chars.len() && chars[k] == 'x' {
                k += 1;
                while k < chars.len() && (chars[k].is_ascii_hexdigit() || chars[k] == '_') { k += 1; }
            } else if k < chars.len() && chars[k] == 'b' {
                k += 1;
                while k < chars.len() && (chars[k] == '0' || chars[k] == '1' || chars[k] == '_') { k += 1; }
            } else if k < chars.len() && chars[k] == 'o' {
                k += 1;
                while k < chars.len() && (('0'..='7').contains(&chars[k]) || chars[k] == '_') { k += 1; }
            } else {
                while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '_') { k += 1; }
            }
            push(&mut tokens, i, k, TokenKind::Number);
            i = k;
            after_comma = false;
            continue;
        }
        // Vector register incl. arrangement / lane index.
        if matches!(c, 'v' | 'q' | 'd' | 's' | 'h' | 'b' | 'z' | 'p')
            && i + 1 < chars.len()
            && chars[i + 1].is_ascii_digit()
        {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() { j += 1; }
            let mut end = j;
            if j < chars.len() && chars[j] == '.' {
                let mut k = j + 1;
                while k < chars.len() && (chars[k].is_ascii_alphanumeric()) { k += 1; }
                if k > j + 1 {
                    let candidate: String = chars[i..k].iter().collect();
                    if parse_vreg(&candidate).is_some() {
                        end = k;
                    }
                }
            }
            if end < chars.len() && chars[end] == '[' {
                let mut k = end + 1;
                while k < chars.len() && chars[k].is_ascii_digit() { k += 1; }
                if k < chars.len() && chars[k] == ']' {
                    let candidate: String = chars[i..=k].iter().collect();
                    if parse_vreg(&candidate).is_some() {
                        end = k + 1;
                    }
                }
            }
            // SVE predicate qualifier /z or /m.
            if end + 1 < chars.len() && chars[end] == '/' && (chars[end + 1] == 'z' || chars[end + 1] == 'm') {
                end += 2;
            }
            let candidate: String = chars[i..end].iter().collect();
            if parse_vreg(&candidate).is_some() {
                push(&mut tokens, i, end, TokenKind::VectorRegister);
                i = end;
                expect_insn = false;
                after_comma = false;
                continue;
            }
        }
        if is_ident_start(c) {
            let mut j = i + 1;
            while j < chars.len() && is_ident_continue(chars[j]) { j += 1; }
            let ident: String = chars[i..j].iter().collect();
            if j < chars.len() && chars[j] == ':' {
                push(&mut tokens, i, j + 1, TokenKind::LabelDecl);
                i = j + 1;
                expect_insn = true;
                continue;
            }
            let lower = ident.to_ascii_lowercase();
            if expect_insn {
                let cond_branch = lower.starts_with("b.") && condition_code(&lower[2..]).is_some();
                if is_known_mnemonic(&lower) || cond_branch {
                    push(&mut tokens, i, j, TokenKind::Instruction);
                } else if lower.starts_with('.')
                    && (DIRECTIVES.contains(&lower.as_str()) || !lower.starts_with(".l"))
                {
                    push(&mut tokens, i, j, TokenKind::Directive);
                } else {
                    push(&mut tokens, i, j, TokenKind::Invalid);
                }
                i = j;
                expect_insn = false;
                after_comma = false;
                continue;
            }
            if lower.starts_with('.') && DIRECTIVES.contains(&lower.as_str()) {
                push(&mut tokens, i, j, TokenKind::Directive);
                i = j;
                continue;
            }
            if parse_gpr(&ident).is_some() {
                push(&mut tokens, i, j, TokenKind::Register);
            } else if parse_vreg(&ident).is_some() {
                push(&mut tokens, i, j, TokenKind::VectorRegister);
            } else if condition_code(&lower).is_some() && (after_comma || true) {
                push(&mut tokens, i, j, TokenKind::Condition);
            } else {
                push(&mut tokens, i, j, TokenKind::LabelRef);
            }
            i = j;
            after_comma = false;
            continue;
        }
        // Punctuation and operators.
        if matches!(c, ',' | '[' | ']' | '{' | '}' | '(' | ')' | '!' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '~' | '<' | '>' | '=' | ':' | ';') {
            push(&mut tokens, i, i + 1, TokenKind::Operator);
            after_comma = c == ',';
            i += 1;
            continue;
        }
        // Anything else is not valid GNU AArch64 syntax.
        push(&mut tokens, i, i + 1, TokenKind::Invalid);
        i += 1;
    }

    Line { text: text.to_string(), tokens, start_in_block: state.in_block_comment, end_in_block: in_block }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '$'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$'
}

fn find_sub(chars: &[char], from: usize, pat: &[char; 2]) -> Option<usize> {
    if from >= chars.len() {
        return None;
    }
    let mut i = from;
    while i + 1 < chars.len() {
        if chars[i] == pat[0] && chars[i + 1] == pat[1] {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<TokenKind> {
        lex_line(text, LexState::default()).tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn basic_instruction() {
        assert_eq!(kinds("mov x0, #42"), vec![
            TokenKind::Instruction, TokenKind::Register, TokenKind::Operator, TokenKind::Immediate
        ]);
    }

    #[test]
    fn label_and_comment() {
        assert_eq!(kinds("loop: b loop // go"), vec![
            TokenKind::LabelDecl, TokenKind::Instruction, TokenKind::LabelRef, TokenKind::Comment
        ]);
    }

    #[test]
    fn vector_registers() {
        assert_eq!(kinds("add v0.16b, v1.16b, v2.16b"), vec![
            TokenKind::Instruction, TokenKind::VectorRegister, TokenKind::Operator,
            TokenKind::VectorRegister, TokenKind::Operator, TokenKind::VectorRegister
        ]);
    }

    #[test]
    fn string_with_comment() {
        assert_eq!(kinds(".asciz \"a // b\""), vec![TokenKind::Directive, TokenKind::String]);
    }

    #[test]
    fn block_comment_state() {
        let l1 = lex_line("/* start", LexState::default());
        assert!(l1.end_in_block);
        let l2 = lex_line("end */ mov x0, #1", LexState { in_block_comment: true });
        assert!(!l2.end_in_block);
        assert_eq!(l2.tokens[0].kind, TokenKind::Comment);
        assert_eq!(l2.tokens[1].kind, TokenKind::Instruction);
    }

    #[test]
    fn incremental_edit() {
        let mut s = HighlightSession::new();
        s.set_text("mov x0, #1\nadd x1, x0, #2\nret\n");
        assert_eq!(s.line_count(), 4);
        s.set_text("mov x0, #1\nadd x1, x0, #3\nret\n");
        assert_eq!(s.line_count(), 4);
        assert!(s.token_string(1, 4).contains(":6\n"));
    }

    #[test]
    fn utf16_offsets() {
        let l = lex_line("// ππ\n", LexState::default());
        assert_eq!(l.tokens[0].start, 0);
        // The comment token spans the whole line, including the newline.
        assert_eq!(l.tokens[0].end, 6);
    }
}
