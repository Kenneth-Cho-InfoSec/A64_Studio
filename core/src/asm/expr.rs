//! Integer expression evaluator for assembler operands and directives.
//!
//! Supports the subset GNU as uses in practice: decimal/hex/binary/octal
//! literals, symbols, `.` (current location), unary + - ~, and the binary
//! operators * / % + - << >> & ^ |, with parentheses.

#[derive(Debug, Clone)]
pub struct ExprError {
    pub message: String,
    pub col: usize,
}

pub trait SymbolResolver {
    fn resolve(&self, name: &str) -> Option<i64>;
    fn location(&self) -> i64;
}

pub struct NoSymbols {
    pub loc: i64,
}
impl SymbolResolver for NoSymbols {
    fn resolve(&self, _name: &str) -> Option<i64> {
        None
    }
    fn location(&self) -> i64 {
        self.loc
    }
}

pub fn eval(expr: &str, resolver: &dyn SymbolResolver) -> Result<i64, ExprError> {
    let mut p = Parser { s: expr.as_bytes(), text: expr, pos: 0, resolver };
    p.skip_ws();
    if p.pos >= p.s.len() {
        return Err(ExprError { message: "expected an expression".into(), col: 0 });
    }
    let v = p.parse_or()?;
    p.skip_ws();
    if p.pos < p.s.len() {
        return Err(ExprError { message: format!("unexpected '{}' in expression", &expr[p.pos..]), col: p.pos });
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    resolver: &'a dyn SymbolResolver,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() && (self.s[self.pos] as char).is_whitespace() {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }
    fn eat(&mut self, b: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ExprError> {
        Err(ExprError { message: msg.into(), col: self.pos })
    }

    fn parse_or(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_xor()?;
        loop {
            self.skip_ws();
            if self.peek() == Some(b'|') && self.s.get(self.pos + 1) != Some(&b'|') {
                self.pos += 1;
                v |= self.parse_xor()?;
            } else {
                return Ok(v);
            }
        }
    }
    fn parse_xor(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_and()?;
        loop {
            self.skip_ws();
            if self.peek() == Some(b'^') {
                self.pos += 1;
                v ^= self.parse_and()?;
            } else {
                return Ok(v);
            }
        }
    }
    fn parse_and(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_shift()?;
        loop {
            self.skip_ws();
            if self.peek() == Some(b'&') && self.s.get(self.pos + 1) != Some(&b'&') {
                self.pos += 1;
                v &= self.parse_shift()?;
            } else {
                return Ok(v);
            }
        }
    }
    fn parse_shift(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_add()?;
        loop {
            self.skip_ws();
            if self.peek() == Some(b'<') && self.s.get(self.pos + 1) == Some(&b'<') {
                self.pos += 2;
                v = v.wrapping_shl(self.parse_add()? as u32 & 63);
            } else if self.peek() == Some(b'>') && self.s.get(self.pos + 1) == Some(&b'>') {
                self.pos += 2;
                v = v.wrapping_shr(self.parse_add()? as u32 & 63);
            } else {
                return Ok(v);
            }
        }
    }
    fn parse_add(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_mul()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    v = v.wrapping_add(self.parse_mul()?);
                }
                Some(b'-') => {
                    self.pos += 1;
                    v = v.wrapping_sub(self.parse_mul()?);
                }
                _ => return Ok(v),
            }
        }
    }
    fn parse_mul(&mut self) -> Result<i64, ExprError> {
        let mut v = self.parse_unary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    v = v.wrapping_mul(self.parse_unary()?);
                }
                Some(b'/') => {
                    self.pos += 1;
                    let d = self.parse_unary()?;
                    if d == 0 {
                        return self.err("division by zero in expression");
                    }
                    v = v.wrapping_div(d);
                }
                Some(b'%') => {
                    self.pos += 1;
                    let d = self.parse_unary()?;
                    if d == 0 {
                        return self.err("modulo by zero in expression");
                    }
                    v = v.wrapping_rem(d);
                }
                _ => return Ok(v),
            }
        }
    }
    fn parse_unary(&mut self) -> Result<i64, ExprError> {
        self.skip_ws();
        match self.peek() {
            Some(b'-') => {
                self.pos += 1;
                Ok(self.parse_unary()?.wrapping_neg())
            }
            Some(b'+') => {
                self.pos += 1;
                self.parse_unary()
            }
            Some(b'~') => {
                self.pos += 1;
                Ok(!self.parse_unary()?)
            }
            Some(b'(') => {
                self.pos += 1;
                let v = self.parse_or()?;
                if !self.eat(b')') {
                    return self.err("missing ')' in expression");
                }
                Ok(v)
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> Result<i64, ExprError> {
        self.skip_ws();
        let start = self.pos;
        let c = match self.peek() {
            Some(c) => c,
            None => return self.err("unexpected end of expression"),
        };
        if c.is_ascii_digit() {
            // 1f / 1b numeric local label references.
            let mut j = self.pos;
            while j < self.s.len() && self.s[j].is_ascii_digit() {
                j += 1;
            }
            if j < self.s.len() && (self.s[j] == b'f' || self.s[j] == b'b') {
                let name = &self.text[self.pos..j + 1];
                self.pos = j + 1;
                return match self.resolver.resolve(name) {
                    Some(v) => Ok(v),
                    None => self.err(format!("undefined local label '{}'", name)),
                };
            }
            let (v, end) = parse_number(self.text, self.pos)?;
            self.pos = end;
            return Ok(v);
        }
        if c == b'.' {
            // Current location, unless this is a dotted symbol.
            if self.pos + 1 >= self.s.len() || !is_sym_cont(self.s[self.pos + 1] as char) {
                self.pos += 1;
                return Ok(self.resolver.location());
            }
        }
        if is_sym_start(c as char) {
            let mut j = self.pos + 1;
            while j < self.s.len() && is_sym_cont(self.s[j] as char) {
                j += 1;
            }
            let name = &self.text[start..j];
            self.pos = j;
            return match self.resolver.resolve(name) {
                Some(v) => Ok(v),
                None => self.err(format!("undefined symbol '{}'", name)),
            };
        }
        self.err(format!("unexpected character '{}' in expression", c as char))
    }
}

fn is_sym_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '.' || c == '$'
}
fn is_sym_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$'
}

fn parse_number(s: &str, start: usize) -> Result<(i64, usize), ExprError> {
    let b = s.as_bytes();
    let mut i = start;
    let (radix, mut value) = if b[i] == b'0' && i + 1 < b.len() {
        match b[i + 1] {
            b'x' | b'X' => (16u32, 0i64),
            b'b' | b'B' => (2, 0),
            b'o' | b'O' => (8, 0),
            _ => (10, 0),
        }
    } else {
        (10, 0)
    };
    if radix != 10 {
        i += 2;
    }
    let digits_start = i;
    while i < b.len() {
        let c = b[i] as char;
        if c == '_' {
            i += 1;
            continue;
        }
        if let Some(d) = c.to_digit(radix) {
            value = value.wrapping_mul(radix as i64).wrapping_add(d as i64);
            i += 1;
        } else {
            break;
        }
    }
    if i == digits_start {
        return Err(ExprError { message: "malformed number".into(), col: start });
    }
    // Hex constants are commonly written as full 64-bit patterns.
    if radix == 16 && i - digits_start == 16 {
        // value already wrapped into i64 correctly
    }
    Ok((value, i))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct S;
    impl SymbolResolver for S {
        fn resolve(&self, n: &str) -> Option<i64> {
            match n {
                "foo" => Some(10),
                "bar" => Some(3),
                _ => None,
            }
        }
        fn location(&self) -> i64 { 0x1000 }
    }
    fn ev(s: &str) -> i64 { eval(s, &S).unwrap() }
    #[test]
    fn arithmetic() {
        assert_eq!(ev("1 + 2 * 3"), 7);
        assert_eq!(ev("(1 + 2) * 3"), 9);
        assert_eq!(ev("foo - bar"), 7);
        assert_eq!(ev("0xff & 0x0f"), 15);
        assert_eq!(ev("1 << 12"), 4096);
        assert_eq!(ev("-1"), -1);
        assert_eq!(ev("."), 0x1000);
    }
    #[test]
    fn errors() {
        assert!(eval("1 +", &S).is_err());
        assert!(eval("missing", &S).is_err());
        assert!(eval("1/0", &S).is_err());
    }
}
