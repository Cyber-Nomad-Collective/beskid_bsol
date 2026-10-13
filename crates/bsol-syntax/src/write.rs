//! Deterministic semantic writing; source-preserving edits are a separate API.
//!
//! Writing follows native `Core.Bsol.Writer` budgets: an invalid descriptor, a graph over the node,
//! depth or scalar budget fails as `InvalidSyntax`, and output that would exceed `maxOutputBytes`
//! fails as `OutputLimit` before it is appended, whichever comes first in emission order. Accepted
//! output is reparsed under the output budget.
use crate::*;

const GRAPH: &str = "invalid or over-limit syntax graph";

/// Write an ordered syntax document under the default [`BsolLimits`].
pub fn write_bsol_document(document: &BsolDocument) -> Result<String, BsolError> {
    write_bsol_document_with_limits(document, &BsolLimits::default())
}

/// Write an ordered syntax document with two-space indentation and a final LF.
pub fn write_bsol_document_with_limits(
    document: &BsolDocument,
    limits: &BsolLimits,
) -> Result<String, BsolError> {
    let mut output = Output::new(limits);
    let nodes = document.blocks.iter().map(block_nodes).sum::<usize>();
    if !limits.is_valid() || nodes > limits.max_nodes {
        output.fail_graph();
    }
    for block in &document.blocks {
        output.block(block, 0, 1);
        output.put("\n");
    }
    if document.blocks.is_empty() {
        output.put("\n");
    }
    let output = output.finish()?;
    for block in &document.blocks {
        check_block(block)?;
    }
    parse_bsol_document_with_limits(&output, &limits.output_parse_limits())?;
    Ok(output)
}

/// Write one value under the default budgets, retaining nested syntax and exact numeric spelling.
pub fn write_bsol_value(value: &BsolValue) -> Result<String, BsolError> {
    let limits = BsolLimits::default();
    let mut output = Output::new(&limits);
    output.value(value, 0, 1);
    let output = output.finish()?;
    check_value(value)?;
    // Validate public, manually constructed AST values as well as parsed ones.
    parse_bsol_document_with_limits(
        &format!("value {{ data = {output} }}"),
        &limits.output_parse_limits(),
    )?;
    Ok(output)
}

/// Native syntax node count of a block: the block, its attributes and arguments, and its body.
fn block_nodes(block: &BsolBlock) -> usize {
    1 + attribute_nodes(&block.attrs)
        + block
            .items
            .iter()
            .map(|item| match item {
                BsolItem::Assignment(a) => 1 + attribute_nodes(&a.attrs) + value_nodes(&a.value),
                BsolItem::Block(b) => block_nodes(b),
            })
            .sum::<usize>()
}
fn attribute_nodes(attrs: &[BsolAttribute]) -> usize {
    attrs
        .iter()
        .map(|attr| {
            1 + attr
                .args
                .iter()
                .map(|arg| 1 + value_nodes(&arg.value))
                .sum::<usize>()
        })
        .sum()
}
fn value_nodes(value: &BsolValue) -> usize {
    match value {
        BsolValue::InlineBlock(b) => block_nodes(b),
        BsolValue::InlineMap(map) => map_nodes(map),
        BsolValue::BracketList(list) => {
            1 + list
                .items
                .iter()
                .map(|item| match item {
                    BsolListItem::InlineMap(map) => map_nodes(map),
                    BsolListItem::InlineBlock(b) => block_nodes(b),
                    _ => 1,
                })
                .sum::<usize>()
        }
        _ => 1,
    }
}
fn map_nodes(map: &BsolInlineMap) -> usize {
    1 + map
        .entries
        .iter()
        .map(|e| 1 + value_nodes(&e.value))
        .sum::<usize>()
}

/// Bytes of the JSON-style quoted form `quoted` produces, counted before allocation.
fn quoted_len(text: &str) -> usize {
    2 + text
        .bytes()
        .map(|byte| match byte {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 0x08 | 0x0c => 2,
            0..=0x1f => 6,
            _ => 1,
        })
        .sum::<usize>()
}
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("string serialization cannot fail")
}

struct Output<'l> {
    text: String,
    limits: &'l BsolLimits,
    failure: Option<BsolLimitKind>,
}

impl<'l> Output<'l> {
    fn new(limits: &'l BsolLimits) -> Self {
        Self {
            text: String::new(),
            limits,
            failure: None,
        }
    }

    fn finish(self) -> Result<String, BsolError> {
        match self.failure {
            Some(kind) => Err(BsolError::limit(kind, GRAPH, 1, 0)),
            None => Ok(self.text),
        }
    }

    fn fail_graph(&mut self) {
        self.failure.get_or_insert(BsolLimitKind::InvalidSyntax);
    }

    /// Append within the output budget; the first overflow is an `OutputLimit` failure.
    fn put(&mut self, text: &str) {
        if self.failure.is_some() {
            return;
        }
        if text.len() > self.limits.max_output_bytes - self.text.len() {
            self.failure = Some(BsolLimitKind::OutputLimit);
            return;
        }
        self.text.push_str(text);
    }

    fn put_quoted(&mut self, text: &str) {
        if self.failure.is_some() {
            return;
        }
        if quoted_len(text) > self.limits.max_output_bytes - self.text.len() {
            self.failure = Some(BsolLimitKind::OutputLimit);
            return;
        }
        self.text.push_str(&quoted(text));
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.put("  ");
        }
    }

    /// Native `Emit` admission of one node at `depth` carrying `scalars`.
    fn node(&mut self, depth: usize, scalars: &[&str]) -> bool {
        if self.failure.is_some() {
            return false;
        }
        if depth > self.limits.max_depth
            || scalars
                .iter()
                .any(|scalar| scalar.len() > self.limits.max_scalar_bytes)
        {
            self.fail_graph();
            return false;
        }
        true
    }

    fn attributes(&mut self, attrs: &[BsolAttribute], indent: usize, depth: usize) {
        for attr in attrs {
            if !self.node(depth, &[&attr.name]) {
                return;
            }
            self.indent(indent);
            self.put("@");
            self.put(&attr.name);
            if !attr.args.is_empty() {
                self.put("(");
                for (index, arg) in attr.args.iter().enumerate() {
                    if index > 0 {
                        self.put(", ");
                    }
                    if !self.node(depth + 1, &[&arg.key]) {
                        return;
                    }
                    self.put(&arg.key);
                    self.put(" = ");
                    self.value(&arg.value, indent, depth + 2);
                }
                self.put(")");
            }
            self.put("\n");
        }
    }

    fn block(&mut self, block: &BsolBlock, indent: usize, depth: usize) {
        let label = block
            .label
            .as_ref()
            .map_or("", |label| label.value.as_str());
        if !self.node(depth, &[&block.kind, label]) {
            return;
        }
        self.attributes(&block.attrs, indent, depth + 1);
        if !block.attrs.is_empty() {
            self.indent(indent);
        }
        self.put(&block.kind);
        if let Some(label) = &block.label {
            self.put(" ");
            self.put_quoted(&label.value);
        }
        if let Some(body) = &block.schemaless_body {
            self.put(" @schemaless {");
            self.put(body);
            self.put("}");
            return;
        }
        self.put(" {\n");
        for item in &block.items {
            match item {
                BsolItem::Assignment(a) => {
                    if !self.node(depth + 1, &[&a.key]) {
                        return;
                    }
                    self.attributes(&a.attrs, indent + 1, depth + 2);
                    self.indent(indent + 1);
                    self.put(&a.key);
                    self.put(" = ");
                    self.value(&a.value, indent + 1, depth + 2);
                    self.put("\n");
                }
                BsolItem::Block(b) => {
                    if b.attrs.is_empty() {
                        self.indent(indent + 1);
                    }
                    self.block(b, indent + 1, depth + 1);
                    self.put("\n");
                }
            }
        }
        self.indent(indent);
        self.put("}");
    }

    fn value(&mut self, value: &BsolValue, indent: usize, depth: usize) {
        match value {
            BsolValue::QuotedString(q) => self.quoted_node(&q.value, depth),
            BsolValue::Ident(text) => self.text_node(text, depth),
            BsolValue::Number(n) => self.text_node(&n.raw, depth),
            BsolValue::Bool(b) => self.text_node(if *b { "true" } else { "false" }, depth),
            BsolValue::Ref(r) => self.reference(r, depth),
            BsolValue::InlineBlock(b) => self.block(b, indent, depth),
            BsolValue::InlineMap(map) => self.map(map, indent, depth),
            BsolValue::BracketList(list) => {
                if !self.node(depth, &[]) {
                    return;
                }
                self.put("[");
                for (index, item) in list.items.iter().enumerate() {
                    if index > 0 {
                        self.put(", ");
                    }
                    match item {
                        BsolListItem::Default => self.text_node("default", depth + 1),
                        BsolListItem::QuotedString(q) => self.quoted_node(&q.value, depth + 1),
                        BsolListItem::Ident(text) => self.text_node(text, depth + 1),
                        BsolListItem::Number(n) => self.text_node(&n.raw, depth + 1),
                        BsolListItem::Bool(b) => {
                            self.text_node(if *b { "true" } else { "false" }, depth + 1)
                        }
                        BsolListItem::Ref(r) => self.reference(r, depth + 1),
                        BsolListItem::InlineMap(map) => self.map(map, indent, depth + 1),
                        BsolListItem::InlineBlock(b) => self.block(b, indent, depth + 1),
                    }
                }
                self.put("]");
            }
        }
    }

    fn text_node(&mut self, text: &str, depth: usize) {
        if self.node(depth, &[text]) {
            self.put(text);
        }
    }

    fn quoted_node(&mut self, text: &str, depth: usize) {
        if self.node(depth, &[text]) {
            self.put_quoted(text);
        }
    }

    fn reference(&mut self, reference: &BsolRef, depth: usize) {
        let qualifier = reference.rule_kind.as_deref().unwrap_or("");
        if self.node(depth, &[qualifier, &reference.label]) {
            self.put(&reference.display());
        }
    }

    fn map(&mut self, map: &BsolInlineMap, indent: usize, depth: usize) {
        if !self.node(depth, &[]) {
            return;
        }
        self.put("{");
        for (index, entry) in map.entries.iter().enumerate() {
            if index > 0 {
                self.put(", ");
            }
            if !self.node(depth + 1, &[&entry.key]) {
                return;
            }
            self.put(&entry.key);
            self.put(" = ");
            self.value(&entry.value, indent, depth + 2);
        }
        self.put("}");
    }
}

fn check_block(block: &BsolBlock) -> Result<(), BsolError> {
    for attribute in &block.attrs {
        for arg in &attribute.args {
            check_value(&arg.value)?;
        }
    }
    for item in &block.items {
        match item {
            BsolItem::Assignment(a) => {
                for attribute in &a.attrs {
                    for arg in &attribute.args {
                        check_value(&arg.value)?;
                    }
                }
                check_value(&a.value)?;
            }
            BsolItem::Block(b) => check_block(b)?,
        }
    }
    Ok(())
}
fn check_value(value: &BsolValue) -> Result<(), BsolError> {
    match value {
        BsolValue::Number(n) => {
            use pest::Parser;
            let valid = BsolParser::parse(Rule::number, &n.raw)
                .ok()
                .and_then(|mut pairs| pairs.next())
                .is_some_and(|pair| pair.as_span().end() == n.raw.len());
            let finite =
                !n.raw.contains(['.', 'e', 'E']) || n.raw.parse::<f64>().is_ok_and(f64::is_finite);
            if !valid || !finite {
                return Err(BsolError::parse_at(n.span, "invalid numeric AST value"));
            }
        }
        BsolValue::InlineBlock(b) => check_block(b)?,
        BsolValue::InlineMap(map) => {
            for entry in &map.entries {
                check_value(&entry.value)?;
            }
        }
        BsolValue::BracketList(list) => {
            for item in &list.items {
                match item {
                    BsolListItem::Number(n) => check_value(&BsolValue::Number(n.clone()))?,
                    BsolListItem::InlineBlock(b) => check_block(b)?,
                    BsolListItem::InlineMap(map) => {
                        for entry in &map.entries {
                            check_value(&entry.value)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Ok(())
}
