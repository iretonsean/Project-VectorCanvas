//! Lexer for the terse form. See GRAMMAR.md §2.

use crate::diag::{Diagnostic, Edit, Span, E_PARSE};

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// Identifier / bare word: `[A-Za-z_#][A-Za-z0-9_.\-/@#]*` (a `-`/`.`/`/`/`@` must be followed by an alphanumeric).
    Ident(String),
    /// `@key` string-table reference.
    Key(String),
    /// Decimal number literal (`320`, `1.5`), kept as text.
    Num(String),
    Str(String),
    /// Punctuation: `{ } ( ) [ ] ; , = : == != <= >= < > + - * ..`
    P(&'static str),
    Nl,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '#'
}
fn is_ident_cont(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '#'
}

pub fn lex(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let (mut line, mut col) = (1u32, 1u32);
    let mut out = Vec::new();
    // Bracket depth: newlines inside ( ) and [ ] are whitespace.
    let mut depth = 0i32;
    macro_rules! adv {
        () => {{
            let c = chars[i];
            i += 1;
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            c
        }};
    }
    while i < chars.len() {
        let c = chars[i];
        let span = Span { line, col };
        if c == '\n' {
            adv!();
            if depth <= 0 {
                out.push(Token { tok: Tok::Nl, span });
            }
            continue;
        }
        if c == ' ' || c == '\t' || c == '\r' {
            adv!();
            continue;
        }
        if c == '\\' && i + 1 < chars.len() && (chars[i + 1] == '\n' || (chars[i + 1] == '\r')) {
            adv!();
            while i < chars.len() && (chars[i] == '\r' || chars[i] == '\n') {
                adv!();
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                adv!();
            }
            continue;
        }
        if c == '"' {
            adv!();
            let mut s = String::new();
            loop {
                if i >= chars.len() || chars[i] == '\n' {
                    return Err(Diagnostic::new(E_PARSE, "parse.unterminated-string")
                        .at_span(span)
                        .param("expected", "closing-quote")
                        .fix("insert", Edit::Expect { expected: "\"".into() }));
                }
                let d = adv!();
                match d {
                    '"' => break,
                    '\\' => {
                        if i >= chars.len() {
                            return Err(Diagnostic::new(E_PARSE, "parse.bad-escape").at_span(span));
                        }
                        let e = adv!();
                        match e {
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'u' => {
                                let ok = i < chars.len() && chars[i] == '{';
                                if !ok {
                                    return Err(Diagnostic::new(E_PARSE, "parse.bad-escape")
                                        .at_span(span)
                                        .param("expected", "\\u{hex}"));
                                }
                                adv!();
                                let mut hex = String::new();
                                while i < chars.len() && chars[i] != '}' && chars[i] != '\n' {
                                    hex.push(adv!());
                                }
                                if i >= chars.len() || chars[i] != '}' {
                                    return Err(Diagnostic::new(E_PARSE, "parse.bad-escape").at_span(span));
                                }
                                adv!();
                                let cp = u32::from_str_radix(&hex, 16)
                                    .ok()
                                    .and_then(char::from_u32);
                                match cp {
                                    Some(ch) => s.push(ch),
                                    None => {
                                        return Err(Diagnostic::new(E_PARSE, "parse.bad-escape")
                                            .at_span(span)
                                            .param("found", hex))
                                    }
                                }
                            }
                            other => {
                                return Err(Diagnostic::new(E_PARSE, "parse.bad-escape")
                                    .at_span(span)
                                    .param("found", other.to_string()))
                            }
                        }
                    }
                    d => s.push(d),
                }
            }
            out.push(Token { tok: Tok::Str(s), span });
            continue;
        }
        if c == '@' {
            adv!();
            let mut k = String::new();
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || "_.-/:".contains(chars[i]))
            {
                k.push(adv!());
            }
            if k.is_empty() {
                return Err(Diagnostic::new(E_PARSE, "parse.expected")
                    .at_span(span)
                    .param("expected", "string-key")
                    .param("found", "@"));
            }
            out.push(Token { tok: Tok::Key(k), span });
            continue;
        }
        if c.is_ascii_digit() {
            let mut n = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                n.push(adv!());
            }
            // A '.' followed by a digit continues the number; ".." is a range.
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                n.push(adv!());
                while i < chars.len() && chars[i].is_ascii_digit() {
                    n.push(adv!());
                }
            }
            // Digits followed directly by letters form a bare word (e.g. a hex digest `5c12ab..`).
            if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
                while i < chars.len() && is_ident_cont(chars[i]) {
                    n.push(adv!());
                }
                out.push(Token { tok: Tok::Ident(n), span });
                continue;
            }
            out.push(Token { tok: Tok::Num(n), span });
            continue;
        }
        if is_ident_start(c) {
            let mut id = String::new();
            id.push(adv!());
            loop {
                if i >= chars.len() {
                    break;
                }
                let d = chars[i];
                if is_ident_cont(d) {
                    id.push(adv!());
                } else if "-./@:".contains(d)
                    && i + 1 < chars.len()
                    && (chars[i + 1].is_ascii_alphanumeric() || chars[i + 1] == '_')
                    && !(d == ':' )
                    && !(d == '.' && chars[i + 1] == '.')
                {
                    id.push(adv!());
                } else {
                    break;
                }
            }
            out.push(Token { tok: Tok::Ident(id), span });
            continue;
        }
        // punctuation (maximal munch)
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let p2: Option<&'static str> = match two.as_str() {
            "==" => Some("=="),
            "!=" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            ".." => Some(".."),
            _ => None,
        };
        if let Some(p) = p2 {
            adv!();
            adv!();
            out.push(Token { tok: Tok::P(p), span });
            continue;
        }
        let p1: Option<&'static str> = match c {
            '{' => Some("{"),
            '}' => Some("}"),
            '(' => Some("("),
            ')' => Some(")"),
            '[' => Some("["),
            ']' => Some("]"),
            ';' => Some(";"),
            ',' => Some(","),
            '=' => Some("="),
            ':' => Some(":"),
            '<' => Some("<"),
            '>' => Some(">"),
            '+' => Some("+"),
            '-' => Some("-"),
            '*' => Some("*"),
            _ => None,
        };
        match p1 {
            Some(p) => {
                adv!();
                if p == "(" || p == "[" {
                    depth += 1;
                } else if p == ")" || p == "]" {
                    depth -= 1;
                }
                out.push(Token { tok: Tok::P(p), span });
            }
            None => {
                return Err(Diagnostic::new(E_PARSE, "parse.unexpected-char")
                    .at_span(span)
                    .param("found", c.to_string()));
            }
        }
    }
    out.push(Token { tok: Tok::Eof, span: Span { line, col } });
    Ok(out)
}
