use crate::BsolError;
use bsol_syntax::{BsolAssignment, BsolValue};

pub(super) fn extra_field_value(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Number(n) => Ok(n.raw.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        BsolValue::Ref(r) => Ok(r.display()),
        structured => bsol_syntax::write_bsol_value(structured).map_err(BsolError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsol_syntax::{parse_bsol_document, BsolItem};

    #[test]
    fn structured_extra_field_format_preserves_nested_values_and_escapes() {
        let source = r#"config { data = [value "record" { nested = value "string" { data = "quote\"line\n" } }] }"#;
        let doc = parse_bsol_document(source).unwrap();
        let BsolItem::Assignment(assignment) = &doc.blocks[0].items[0] else {
            panic!("assignment");
        };
        let text = extra_field_value(assignment).unwrap();
        assert!(
            text.contains("quote\\\"line\\n"),
            "lost nested escaped value: {text}"
        );
        let reparsed = parse_bsol_document(&format!("config {{ data = {text} }}")).unwrap();
        let BsolItem::Assignment(a) = &reparsed.blocks[0].items[0] else {
            panic!("assignment");
        };
        assert!(matches!(&a.value, BsolValue::BracketList(list) if list.items.len() == 1));
    }
}
