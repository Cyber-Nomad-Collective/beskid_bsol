//! Generic BSOL abstract syntax tree.

/// A parsed BSOL document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolDocument {
    pub blocks: Vec<BsolBlock>,
}

/// Attribute metadata on blocks or assignments (v2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolAttribute {
    pub span: BsolSpan,
    pub name: String,
    pub args: Vec<BsolAttributeArg>,
}

/// Named argument inside an attribute: `key = value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolAttributeArg {
    pub key: String,
    pub value: BsolValue,
}

/// One block in a BSOL document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolBlock {
    pub span: BsolSpan,
    pub attrs: Vec<BsolAttribute>,
    pub kind: String,
    pub label: Option<BsolQuotedString>,
    /// When `@schemaless` is present, inner `{ ... }` text is captured verbatim (no nested parse).
    pub schemaless_body: Option<String>,
    pub items: Vec<BsolItem>,
}

/// Body item: assignment or nested block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BsolItem {
    Assignment(BsolAssignment),
    Block(BsolBlock),
}

/// `key = value` assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolAssignment {
    pub span: BsolSpan,
    pub attrs: Vec<BsolAttribute>,
    pub key: String,
    pub value: BsolValue,
}

/// Right-hand side of an assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BsolValue {
    QuotedString(BsolQuotedString),
    Ident(String),
    Bool(bool),
    BracketList(BsolBracketList),
    InlineMap(BsolInlineMap),
    Ref(BsolRef),
}

/// Cross-block reference: `@kind/label` or `@/label`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolRef {
    pub span: BsolSpan,
    pub rule_kind: Option<String>,
    pub label: String,
}

/// Inline map `{ key = value, ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolInlineMap {
    pub span: BsolSpan,
    pub entries: Vec<BsolMapEntry>,
}

/// One entry in an inline map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolMapEntry {
    pub span: BsolSpan,
    pub key: String,
    pub value: BsolValue,
}

/// Double-quoted string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolQuotedString {
    pub span: BsolSpan,
    pub value: String,
}

/// Bracket list `[a, b, "c"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BsolBracketList {
    pub span: BsolSpan,
    pub items: Vec<BsolListItem>,
}

/// One element of a bracket list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BsolListItem {
    Default,
    QuotedString(BsolQuotedString),
    Ident(String),
    Bool(bool),
    Ref(BsolRef),
    InlineMap(BsolInlineMap),
    InlineBlock(BsolBlock),
}

/// UTF-8 source span with 1-based line index for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BsolSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

impl BsolSpan {
    pub fn from_pest(span: pest::Span<'_>, source: &str) -> Self {
        let start = span.start();
        let end = span.end();
        let line = source[..start.min(source.len())].lines().count().max(1);
        Self { start, end, line }
    }
}

impl BsolQuotedString {
    pub fn new(span: BsolSpan, raw: &str) -> Self {
        let value = raw
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(raw)
            .to_string();
        Self { span, value }
    }
}

impl BsolValue {
    /// String preview for diagnostics.
    pub fn preview(&self) -> String {
        match self {
            BsolValue::QuotedString(q) => format!("\"{}\"", q.value),
            BsolValue::Ident(i) => i.clone(),
            BsolValue::Bool(b) => b.to_string(),
            BsolValue::BracketList(_) => "[...]".to_string(),
            BsolValue::InlineMap(_) => "{...}".to_string(),
            BsolValue::Ref(r) => r.display(),
        }
    }
}

impl BsolRef {
    pub fn display(&self) -> String {
        match &self.rule_kind {
            Some(kind) => format!("@{kind}/{}", self.label),
            None => format!("@/{}", self.label),
        }
    }
}
