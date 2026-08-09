use bsol_syntax::{BsolAssignment, BsolInlineMap, BsolListItem, BsolValue};

use crate::BsolError;

pub(super) fn extra_field_value(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        BsolValue::Ref(r) => Ok(r.display()),
        BsolValue::BracketList(list) => Ok(format_bracket_list_literal(list)),
        BsolValue::InlineMap(map) => Ok(format_map_literal(map)),
    }
}

fn format_map_literal(map: &BsolInlineMap) -> String {
    let mut out = String::from("{");
    for (index, entry) in map.entries.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&entry.key);
        out.push_str(" = ");
        out.push_str(&entry.value.preview());
    }
    out.push('}');
    out
}

fn format_bracket_list_literal(list: &bsol_syntax::BsolBracketList) -> String {
    let mut out = String::from("[");
    for (index, item) in list.items.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        match item {
            BsolListItem::Default => out.push_str("default"),
            BsolListItem::QuotedString(q) => {
                out.push('"');
                out.push_str(&q.value);
                out.push('"');
            }
            BsolListItem::Ident(i) => out.push_str(i),
            BsolListItem::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            BsolListItem::Ref(r) => out.push_str(&r.display()),
            BsolListItem::InlineMap(m) => out.push_str(&format_map_literal(m)),
            BsolListItem::InlineBlock(b) => {
                out.push_str(&b.kind);
                if let Some(label) = &b.label {
                    out.push('"');
                    out.push_str(&label.value);
                    out.push('"');
                }
                out.push_str(" { ... }");
            }
        }
    }
    out.push(']');
    out
}
