//! Deterministic semantic writing; source-preserving edits are a separate API.
use crate::*;

/// Write an ordered syntax document with two-space indentation and a final LF.
pub fn write_bsol_document(document: &BsolDocument) -> Result<String, BsolError> {
    let mut output = String::new();
    for block in &document.blocks {
        check_block(block)?;
        output.push_str(&write_block(block, 0));
        output.push('\n');
    }
    if output.is_empty() {
        output.push('\n');
    }
    parse_bsol_document(&output)?;
    Ok(output)
}

/// Write one value, retaining nested syntax and exact numeric spelling.
pub fn write_bsol_value(value: &BsolValue) -> Result<String, BsolError> {
    check_value(value)?;
    let output = write_value(value, 0);
    // Validate public, manually constructed AST values as well as parsed ones.
    parse_bsol_document(&format!("value {{ data = {output} }}"))?;
    Ok(output)
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("string serialization cannot fail")
}
fn attributes(attrs: &[BsolAttribute], depth: usize) -> String {
    let mut output = String::new();
    for attr in attrs {
        output.push_str(&"  ".repeat(depth));
        output.push('@');
        output.push_str(&attr.name);
        if !attr.args.is_empty() {
            output.push('(');
            output.push_str(
                &attr
                    .args
                    .iter()
                    .map(|arg| format!("{} = {}", arg.key, write_value(&arg.value, depth)))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            output.push(')');
        }
        output.push('\n');
    }
    output
}
fn write_block(block: &BsolBlock, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut output = attributes(&block.attrs, depth);
    if !block.attrs.is_empty() {
        output.push_str(&indent);
    }
    output.push_str(&block.kind);
    if let Some(label) = &block.label {
        output.push(' ');
        output.push_str(&quoted(&label.value));
    }
    if let Some(body) = &block.schemaless_body {
        output.push_str(" @schemaless {");
        output.push_str(body);
        output.push('}');
        return output;
    }
    output.push_str(" {\n");
    for item in &block.items {
        match item {
            BsolItem::Assignment(a) => {
                output.push_str(&attributes(&a.attrs, depth + 1));
                output.push_str(&"  ".repeat(depth + 1));
                output.push_str(&a.key);
                output.push_str(" = ");
                output.push_str(&write_value(&a.value, depth + 1));
                output.push('\n');
            }
            BsolItem::Block(b) => {
                if b.attrs.is_empty() {
                    output.push_str(&"  ".repeat(depth + 1));
                }
                output.push_str(&write_block(b, depth + 1));
                output.push('\n');
            }
        }
    }
    output.push_str(&indent);
    output.push('}');
    output
}
fn write_value(value: &BsolValue, depth: usize) -> String {
    match value {
        BsolValue::QuotedString(q) => quoted(&q.value),
        BsolValue::Ident(i) => i.clone(),
        BsolValue::Number(n) => n.raw.clone(),
        BsolValue::Bool(b) => b.to_string(),
        BsolValue::Ref(r) => r.display(),
        BsolValue::InlineBlock(b) => write_block(b, depth),
        BsolValue::InlineMap(map) => format!(
            "{{{}}}",
            map.entries
                .iter()
                .map(|entry| format!("{} = {}", entry.key, write_value(&entry.value, depth)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        BsolValue::BracketList(list) => format!(
            "[{}]",
            list.items
                .iter()
                .map(|item| match item {
                    BsolListItem::Default => "default".to_string(),
                    BsolListItem::QuotedString(q) => quoted(&q.value),
                    BsolListItem::Ident(i) => i.clone(),
                    BsolListItem::Number(n) => n.raw.clone(),
                    BsolListItem::Bool(b) => b.to_string(),
                    BsolListItem::Ref(r) => r.display(),
                    BsolListItem::InlineMap(map) =>
                        write_value(&BsolValue::InlineMap(map.clone()), depth),
                    BsolListItem::InlineBlock(block) => write_block(block, depth),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
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
