use super::model::{BsolItem, BsolValue};
use super::parse_bsol_document;

#[test]
fn v06_qualified_import_reference_syntax() {
    let source = "config { target = @shared.node/alpha shape = ref(shared.node) nested = inline(shared.node) }";
    let document = parse_bsol_document(source).expect("qualified imported reference syntax");
    let BsolItem::Assignment(target) = &document.blocks[0].items[0] else {
        panic!("reference assignment");
    };
    let BsolValue::Ref(reference) = &target.value else {
        panic!("reference value");
    };
    assert_eq!(reference.rule_kind.as_deref(), Some("shared.node"));
    assert_eq!(reference.label, "alpha");
    let written = crate::write_bsol_document(&document).expect("qualified reference writes");
    assert!(written.contains("@shared.node/alpha"));
    assert!(written.contains("ref(shared.node)"));
    assert!(written.contains("inline(shared.node)"));
    parse_bsol_document(&written).expect("qualified reference reparses");
    for invalid in [
        "@shared./alpha",
        "@shared..node/alpha",
        "@.node/alpha",
        "ref(shared..node)",
        "inline(shared.)",
    ] {
        assert!(
            parse_bsol_document(&format!("config {{ target = {invalid} }}")).is_err(),
            "invalid qualifier {invalid}"
        );
    }
}

#[test]
fn parse_nested_blocks() {
    let src = r#"p {
  name = "p"
  mod {
    maxGeneratorRounds = 4
  }
}
target "t" {
  kind = Lib
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(doc.blocks[0].kind, "p");
    assert!(matches!(doc.blocks[0].items[1], BsolItem::Block(_)));
}

#[test]
fn parse_schemaless_block_captures_raw_body() {
    let src = r#"raw @schemaless {
  this is not = valid bsol { but it stays }
  nested { braces } ok
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    assert_eq!(doc.blocks.len(), 1);
    let block = &doc.blocks[0];
    assert_eq!(block.kind, "raw");
    let body = block.schemaless_body.as_ref().expect("schemaless body");
    assert!(body.contains("this is not = valid bsol"));
    assert!(body.contains("nested { braces }"));
    assert!(block.items.is_empty());
}

#[test]
fn parse_v2_values() {
    let src = r#"demo {
  enabled = true
  root = @node/main
  env = { DEBUG = "1", PORT = 8080 }
  tags = [a, @node/x, node { kind = panel }]
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    let block = &doc.blocks[0];
    let assign = |key: &str| {
        block
            .items
            .iter()
            .find_map(|item| match item {
                BsolItem::Assignment(a) if a.key == key => Some(a),
                _ => None,
            })
            .expect(key)
    };
    assert!(matches!(assign("enabled").value, BsolValue::Bool(true)));
    assert!(matches!(assign("root").value, BsolValue::Ref(_)));
    assert!(matches!(assign("env").value, BsolValue::InlineMap(_)));
    assert!(matches!(assign("tags").value, BsolValue::BracketList(_)));
}

#[test]
fn parse_attributes() {
    let src = r#"@Deprecated(since = "2.0")
demo {
  name = "demo"
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    assert_eq!(doc.blocks[0].attrs[0].name, "Deprecated");
}
