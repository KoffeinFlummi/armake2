//! A small evaluator for config `__EVAL` / `__EXEC`.
//!
//! Supports number and string literals, `+ - * / % ^`, parentheses, unary minus, string `+`
//! and variables. Anything else (SQF commands, comparisons) is rejected.

use std::collections::HashMap;

use std::io::Error;


/// Result of an `__EVAL`
#[derive(Debug, Clone, PartialEq)]
pub enum EvalValue {
    /// Number
    Number(f32),
    /// String
    Str(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f32),
    Str(String),
    Ident(String),
    Op(char),
}

fn unsupported(expr: &str) -> Error {
    error!("__EVAL expression not supported by armake2: {}", expr)
}

fn tokenize(expr: &str) -> Result<Vec<Tok>, Error> {
    let chars: Vec<char> = expr.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
            let text: String = chars[start..i].iter().collect();
            toks.push(Tok::Num(text.parse().map_err(|_| unsupported(expr))?));
        } else if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                match chars.get(i) {
                    None => return Err(unsupported(expr)),
                    Some('"') if chars.get(i + 1) == Some(&'"') => { s.push('"'); i += 2; },
                    Some('"') => { i += 1; break; },
                    Some(&ch) => { s.push(ch); i += 1; },
                }
            }
            toks.push(Tok::Str(s));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') { i += 1; }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if "+-*/%^()".contains(c) {
            toks.push(Tok::Op(c));
            i += 1;
        } else {
            return Err(unsupported(expr));
        }
    }
    Ok(toks)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    expr: &'a str,
    vars: &'a HashMap<String, EvalValue>,
}

impl<'a> Parser<'a> {
    fn peek_op(&self) -> Option<char> {
        match self.toks.get(self.pos) {
            Some(Tok::Op(c)) => Some(*c),
            _ => None,
        }
    }

    fn prec(op: char) -> Option<u8> {
        match op {
            '+' | '-' => Some(1),
            '*' | '/' | '%' => Some(2),
            '^' => Some(3),
            _ => None,
        }
    }

    fn expr(&mut self, min_prec: u8) -> Result<EvalValue, Error> {
        let mut lhs = self.atom()?;
        while let Some(op) = self.peek_op() {
            let prec = match Self::prec(op) { Some(p) if p >= min_prec => p, _ => break };
            self.pos += 1;
            // `^` is right-associative, everything else left-associative
            let rhs = self.expr(if op == '^' { prec } else { prec + 1 })?;
            lhs = self.apply(op, lhs, rhs)?;
        }
        Ok(lhs)
    }

    fn atom(&mut self) -> Result<EvalValue, Error> {
        let tok = self.toks.get(self.pos).cloned().ok_or_else(|| unsupported(self.expr))?;
        self.pos += 1;
        match tok {
            Tok::Num(n) => Ok(EvalValue::Number(n)),
            Tok::Str(s) => Ok(EvalValue::Str(s)),
            Tok::Ident(name) => self.vars.get(&name).cloned().ok_or_else(|| unsupported(self.expr)),
            Tok::Op('-') => match self.atom()? {
                EvalValue::Number(n) => Ok(EvalValue::Number(-n)),
                _ => Err(unsupported(self.expr)),
            },
            Tok::Op('(') => {
                let v = self.expr(1)?;
                if self.peek_op() != Some(')') { return Err(unsupported(self.expr)); }
                self.pos += 1;
                Ok(v)
            },
            Tok::Op(_) => Err(unsupported(self.expr)),
        }
    }

    fn apply(&self, op: char, a: EvalValue, b: EvalValue) -> Result<EvalValue, Error> {
        match (a, b) {
            (EvalValue::Number(x), EvalValue::Number(y)) => Ok(EvalValue::Number(match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => {
                    if y == 0.0 { return Err(error!("__EVAL: division by zero in: {}", self.expr)); }
                    x / y
                },
                '%' => {
                    if y == 0.0 { return Err(error!("__EVAL: division by zero in: {}", self.expr)); }
                    x % y
                },
                '^' => x.powf(y),
                _ => return Err(unsupported(self.expr)),
            })),
            (EvalValue::Str(x), EvalValue::Str(y)) if op == '+' => Ok(EvalValue::Str(x + &y)),
            _ => Err(unsupported(self.expr)),
        }
    }
}

/// Evaluates an expression.
pub fn eval(expr: &str, vars: &mut HashMap<String, EvalValue>) -> Result<EvalValue, Error> {
    let toks = tokenize(expr)?;
    let mut parser = Parser { toks, pos: 0, expr, vars };
    let value = parser.expr(1)?;
    if parser.pos != parser.toks.len() {
        return Err(unsupported(expr));
    }
    Ok(value)
}

/// Executes `;`-separated assignments (`name = expr`), storing the results in `vars`.
pub fn exec(stmts: &str, vars: &mut HashMap<String, EvalValue>) -> Result<(), Error> {
    let mut in_string = false;
    let mut current = String::new();
    let mut parts = Vec::new();
    for c in stmts.chars() {
        if c == '"' { in_string = !in_string; }
        if c == ';' && !in_string {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);

    for stmt in parts.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let eq = stmt.find('=').ok_or_else(|| error!("__EXEC statement not supported by armake2: {}", stmt))?;
        let name = stmt[..eq].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(error!("__EXEC statement not supported by armake2: {}", stmt));
        }
        let value = eval(stmt[eq + 1..].trim(), vars)?;
        vars.insert(name.to_string(), value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> Result<EvalValue, Error> { eval(s, &mut HashMap::new()) }

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(ev("1+2*3").unwrap(), EvalValue::Number(7.0));
        assert_eq!(ev("2^3^2").unwrap(), EvalValue::Number(512.0));
        assert_eq!(ev("2*3^2").unwrap(), EvalValue::Number(18.0));
        assert_eq!(ev("(1+2)*3/2").unwrap(), EvalValue::Number(4.5));
    }

    #[test]
    fn unsupported_is_error() {
        assert!(ev("floor 2.7").unwrap_err().to_string().contains("not supported"));
        assert!(ev("1 > 0").unwrap_err().to_string().contains("not supported"));
    }
}
