use bsol_syntax::{parse_bsol_document, BsolItem, BsolValue};

fn value(source: &str) -> BsolValue {
    let document = parse_bsol_document(source).expect("valid corrected syntax");
    match document.blocks[0].items[0].clone() {
        BsolItem::Assignment(a) => a.value,
        _ => panic!("assignment expected"),
    }
}

#[test]
fn strict_strings_decode_escapes_and_surrogate_pairs() {
    let parsed = value(r#"config { text = "line\nquote\"slash\\emoji\uD83D\uDE00" }"#);
    match parsed {
        BsolValue::QuotedString(q) => assert_eq!(q.value, "line\nquote\"slash\\emoji😀"),
        _ => panic!("string expected"),
    }
}

#[test]
fn invalid_strings_and_numbers_are_not_silently_accepted() {
    for literal in [
        r#""\q""#,
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uD800\u0041""#,
        "\"raw\nline\"",
        "01",
        "-01",
        "1e999",
        "1e",
        "NaN",
        "Infinity",
        "1.2.3",
    ] {
        let source = format!("config {{ value = {literal} }}");
        assert!(parse_bsol_document(&source).is_err(), "accepted {literal}");
    }
}

#[test]
fn signed_decimals_and_numeric_lists_preserve_exact_spelling() {
    for literal in ["-12", "18446744073709551615", "-0.0", "1.25e+2"] {
        let source = format!("config {{ value = {literal} }}");
        assert_eq!(value(&source).preview(), literal);
    }
    let parsed = value("config { values = [-12, 18446744073709551615, -0.0, 1.25e+2] }");
    assert!(matches!(parsed, BsolValue::BracketList(ref list) if list.items.len() == 4));
}

#[test]
fn empty_maps_and_inline_rhs_blocks_parse_without_losing_items() {
    assert!(
        matches!(value("config { empty = {} }"), BsolValue::InlineMap(ref map) if map.entries.is_empty())
    );
    let document = parse_bsol_document(include_str!("../../../conformance/cases/v06-values.bsol"))
        .expect("shared corpus");
    assert_eq!(document.blocks[0].items.len(), 4);
    assert_eq!(document.blocks[0].attrs.len(), 1);
    assert_eq!(
        document.blocks[0].label.as_ref().unwrap().value,
        "escaped\"label"
    );
    let BsolItem::Assignment(a) = &document.blocks[0].items[0] else {
        panic!("assignment");
    };
    assert_eq!(a.attrs.len(), 1, "attributes must not be duplicated");
}

#[test]
fn canonical_attributes_reject_brackets_and_bad_argument_separators() {
    for source in [
        "[Deprecated] config {}",
        "@A() config {}",
        "@A(x = 1 y = 2) config {}",
        "@A(x = 1,) config {}",
        "config { [A] field = 1 }",
    ] {
        assert!(parse_bsol_document(source).is_err(), "accepted {source}");
    }
    assert!(parse_bsol_document("@A(x = 1, y = true) config { @B field = 2 }").is_ok());
}

#[test]
fn canonical_writer_indents_nested_attributes_and_preserves_all_values() {
    let doc = parse_bsol_document("config { @A nested { text = \"ok\" } }").unwrap();
    let output = bsol_syntax::write_bsol_document(&doc).unwrap();
    assert_eq!(
        output,
        "config {\n  @A\n  nested {\n    text = \"ok\"\n  }\n}\n"
    );
    let corpus =
        parse_bsol_document(include_str!("../../../conformance/cases/v06-values.bsol")).unwrap();
    let canonical = bsol_syntax::write_bsol_document(&corpus).unwrap();
    let again =
        bsol_syntax::write_bsol_document(&parse_bsol_document(&canonical).unwrap()).unwrap();
    assert_eq!(canonical, again);
    assert!(canonical.contains("18446744073709551615"));
    assert!(canonical.contains("data = \"ok\""));
}

#[test]
fn nested_rhs_block_spans_and_reference_spelling_match_source() {
    let source = "config { data = value \"record\" { nested = value \"string\" { data = \"😀\" } child { a = -3 } } ref = @main }";
    let doc = parse_bsol_document(source).unwrap();
    let BsolItem::Assignment(a) = &doc.blocks[0].items[0] else {
        panic!("assignment");
    };
    let BsolValue::InlineBlock(block) = &a.value else {
        panic!("inline block");
    };
    assert_eq!(block.items.len(), 2);
    let BsolItem::Assignment(inner) = &block.items[0] else {
        panic!("nested assignment");
    };
    assert!(source[inner.span.start..inner.span.end].starts_with("nested ="));
    let BsolValue::InlineBlock(nested) = &inner.value else {
        panic!("nested value");
    };
    assert_eq!(
        &source[nested.span.start..nested.span.end],
        "value \"string\" { data = \"😀\" }"
    );
    let BsolItem::Assignment(reference) = &doc.blocks[0].items[1] else {
        panic!("reference");
    };
    assert_eq!(reference.value.preview(), "@main");
}

#[test]
fn pinned_runtime_abi_manifest_roundtrips_without_syntax_loss() {
    let source = include_str!("../../../conformance/cases/runtime-manifest-v5.bsol");
    let doc = parse_bsol_document(source).unwrap();
    assert!(doc.blocks.len() > 100, "complete pinned runtime fixture");
    let canonical = bsol_syntax::write_bsol_document(&doc).unwrap();
    let parsed = parse_bsol_document(&canonical).unwrap();
    assert_eq!(parsed.blocks.len(), doc.blocks.len());
    assert_eq!(
        bsol_syntax::write_bsol_document(&parsed).unwrap(),
        canonical
    );
}

#[test]
fn identifier_prefixes_and_schemaless_raw_text_keep_existing_semantics() {
    assert!(
        matches!(value("config { value = true_identifier }"), BsolValue::Ident(ref i) if i == "true_identifier")
    );
    let doc = parse_bsol_document(r#"raw @schemaless { arbitrary "\q" text }"#).unwrap();
    assert_eq!(
        doc.blocks[0].schemaless_body.as_deref(),
        Some(r#" arbitrary "\q" text "#)
    );
}

#[test]
fn malformed_unicode_body_returns_a_diagnostic_instead_of_panicking() {
    let result = parse_bsol_document("config { 😀 }");
    assert!(result.is_err());
    let error = result.unwrap_err();
    let (start, end) = error.source_span().unwrap();
    assert!("config { 😀 }".is_char_boundary(start));
    assert!("config { 😀 }".is_char_boundary(end));
}

#[test]
fn unicode_error_spans_are_boundaries_and_line_numbers_are_one_based() {
    for source in [
        "😀 {}",
        "config { value = 😀 }",
        "\nconfig {\n  value = -2\n}",
    ] {
        let result = std::panic::catch_unwind(|| parse_bsol_document(source));
        assert!(result.is_ok(), "parser panicked on {source}");
        match result.unwrap() {
            Err(error) => {
                let (start, end) = error.source_span().unwrap();
                assert!(source.is_char_boundary(start) && source.is_char_boundary(end));
            }
            Ok(doc) => {
                assert_eq!(doc.blocks[0].span.line, 2);
                let BsolItem::Assignment(a) = &doc.blocks[0].items[0] else {
                    panic!("assignment");
                };
                assert_eq!(a.span.line, 3);
            }
        }
    }
}

#[test]
fn writer_rejects_injected_or_nonfinite_manually_constructed_number() {
    for raw in ["1 other = 2", "01", "1e999", "NaN"] {
        let number = BsolValue::Number(bsol_syntax::BsolNumber {
            span: Default::default(),
            raw: raw.into(),
        });
        assert!(
            bsol_syntax::write_bsol_value(&number).is_err(),
            "accepted numeric injection {raw}"
        );
    }
}
