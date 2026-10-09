//! Diagnostics (G4 §9.1): code, severity, location, message key + params, fixes.
//! Messages are keys, never English prose (G4 §9.1).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn letter(self) -> char {
        match self {
            Severity::Error => 'E',
            Severity::Warning => 'W',
            Severity::Info => 'I',
        }
    }
}

/// A source position in the terse text (1-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Where a diagnostic applies: node id + IR field path, plus the terse source span when known.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Location {
    pub node: Option<String>,
    pub path: String,
    pub span: Option<Span>,
}

/// A machine-applicable suggestion (G4 §9.1 "fixes"). Structured, not prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// Stable fix key, e.g. `set-name`, `declare-capability`, `choose`.
    pub key: String,
    pub edit: Edit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Set property `prop` on the node at the location to `value` (terse syntax).
    SetProp { prop: String, value: String },
    /// Add a top-level terse line.
    AddLine { line: String },
    /// Replace the offending token with one of these candidates.
    Choose { candidates: Vec<String> },
    /// Add one of the listed fields/props (the "field list for the kind").
    FieldList { fields: Vec<String> },
    /// Insert the expected token class at the location.
    Expect { expected: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub location: Location,
    pub message_key: &'static str,
    pub params: Vec<(String, String)>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message_key: &'static str) -> Self {
        let severity = match code.as_bytes().get(4) {
            Some(b'W') => Severity::Warning,
            Some(b'I') => Severity::Info,
            _ => Severity::Error,
        };
        Diagnostic {
            code,
            severity,
            location: Location::default(),
            message_key,
            params: Vec::new(),
            fixes: Vec::new(),
        }
    }
    pub fn at_span(mut self, span: Span) -> Self {
        self.location.span = Some(span);
        self
    }
    pub fn at_node(mut self, node: &str, path: &str) -> Self {
        self.location.node = Some(node.to_string());
        self.location.path = path.to_string();
        self
    }
    pub fn at_path(mut self, path: &str) -> Self {
        self.location.path = path.to_string();
        self
    }
    pub fn param(mut self, k: &str, v: impl Into<String>) -> Self {
        self.params.push((k.to_string(), v.into()));
        self
    }
    pub fn fix(mut self, key: &str, edit: Edit) -> Self {
        self.fixes.push(Fix { key: key.to_string(), edit });
        self
    }
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

pub const E_PARSE: &str = "PVC-E-PARSE";
pub const E_SCHEMA: &str = "PVC-E-SCHEMA";
pub const E_UNKNOWN_KIND: &str = "PVC-E-UNKNOWN-KIND";
pub const E_UNKNOWN_TOKEN: &str = "PVC-E-UNKNOWN-TOKEN";
pub const E_REF: &str = "PVC-E-REF";
pub const E_ID_COLLISION: &str = "PVC-E-ID-COLLISION";
pub const E_TYPE: &str = "PVC-E-TYPE";
pub const E_CAP_UNDECLARED: &str = "PVC-E-CAP-UNDECLARED";
pub const E_A11Y_NAME: &str = "PVC-E-A11Y-NAME";
pub const E_STRING_MISSING: &str = "PVC-E-STRING-MISSING";

/// Levenshtein-nearest candidates (deterministic: distance, then lexicographic), at most `n`.
pub fn nearest(word: &str, candidates: &[&str], n: usize) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> =
        candidates.iter().map(|c| (lev(word, c), *c)).collect();
    scored.sort();
    scored.into_iter().take(n).map(|(_, c)| c.to_string()).collect()
}

fn lev(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn q(s: &str) -> String {
    if !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '"' || c == '\\') {
        return s.to_string();
    }
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

impl Edit {
    fn render(&self) -> String {
        match self {
            Edit::SetProp { prop, value } => format!("set:{}={}", prop, value),
            Edit::AddLine { line } => format!("add:{}", line),
            Edit::Choose { candidates } => format!("choose:{}", candidates.join("|")),
            Edit::FieldList { fields } => format!("fields:{}", fields.join("|")),
            Edit::Expect { expected } => format!("expect:{}", q(expected)),
        }
    }
}

impl Diagnostic {
    /// Compact, stable one-line text form (the `pvc-validate` output format; see agent-context
    /// and GRAMMAR.md §8): `<E|W|I> <CODE> <line>:<col|-> <message-key> [node=..] [path=..] [k=v ...] [fix=<key>[<edit>] ...]`.
    pub fn render(&self) -> String {
        let mut s = format!(
            "{} {} {} {}",
            self.severity.letter(),
            self.code,
            self.location.span.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
            self.message_key
        );
        if let Some(n) = &self.location.node {
            s.push_str(&format!(" node={}", q(n)));
        }
        if !self.location.path.is_empty() {
            s.push_str(&format!(" path={}", q(&self.location.path)));
        }
        for (k, v) in &self.params {
            s.push_str(&format!(" {}={}", k, q(v)));
        }
        for f in &self.fixes {
            s.push_str(&format!(" fix={}[{}]", f.key, f.edit.render()));
        }
        s
    }
}

impl Diagnostic {
    /// `message-key k=v k=v` (key + params; still a key, never prose).
    pub fn message(&self) -> String {
        let mut s = self.message_key.to_string();
        for (k, v) in &self.params {
            s.push_str(&format!(" {}={}", k, q(v)));
        }
        s
    }
    /// All fixes as `key[edit]`, space separated.
    pub fn fix_hint(&self) -> String {
        self.fixes.iter().map(|f| format!("{}[{}]", f.key, f.edit.render())).collect::<Vec<_>>().join(" ")
    }
}
