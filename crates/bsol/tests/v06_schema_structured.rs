//! Identical structured-schema sources are consumed by Rust and native Corelib.
use bsol::{ValidatedValue, load_profile_from_source, parse_bsol_document, validate};

fn case(name: &str) -> String {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/schema-structured");
    std::fs::read_to_string(directory.join(format!("{name}.bsol"))).unwrap()
}

#[test]
fn shared_structured_values_preserve_nested_types() {
    let profile = load_profile_from_source(&case("profile")).unwrap();
    let document = parse_bsol_document(&case("valid")).unwrap();
    let result = validate(&document, &profile).unwrap();
    let block = &result.blocks[0];
    assert_eq!(
        block.value("title"),
        Some(&ValidatedValue::String("typed".into()))
    );
    assert_eq!(
        block.value("items"),
        Some(&ValidatedValue::List(vec![
            ValidatedValue::String("one".into()),
            ValidatedValue::U32(u32::MAX),
        ]))
    );
    let Some(ValidatedValue::Map(scores)) = block.value("scores") else {
        panic!("scores must remain a typed map")
    };
    assert_eq!(scores.get("alpha"), Some(&ValidatedValue::U32(1)));
    assert_eq!(scores.get("beta"), Some(&ValidatedValue::U32(2)));
    let Some(ValidatedValue::Block(embedded)) = block.value("embedded") else {
        panic!("embedded must remain a typed block")
    };
    assert_eq!(embedded.kind, "item");
    assert_eq!(
        embedded.values.get("name"),
        Some(&ValidatedValue::String("nested".into()))
    );
    let Some(ValidatedValue::Ref(reference)) = block.value("target") else {
        panic!("target must remain a typed reference")
    };
    assert_eq!(reference.rule_kind.as_deref(), Some("item"));
    assert_eq!(reference.label, "one");
}

macro_rules! rejected {
    ($test:ident, $name:literal) => {
        #[test]
        fn $test() {
            let profile = load_profile_from_source(&case("profile")).unwrap();
            let source = case($name);
            let document = parse_bsol_document(&source).unwrap();
            let error = validate(&document, &profile)
                .expect_err(concat!("shared case must reject: ", $name));
            let (start, end) = error
                .manifest_source_span()
                .expect("validation error must retain source bounds");
            assert!(
                start < end && end <= source.len(),
                "invalid source bounds: {error}"
            );
            assert!(
                error.manifest_source_line().is_some(),
                "validation error must retain source line"
            );
        }
    };
}

rejected!(shared_structured_list_rejects_bool, "bad-list");
rejected!(shared_structured_map_rejects_quoted_numeric, "bad-map");
rejected!(
    shared_structured_inline_validates_nested_field,
    "bad-inline"
);
rejected!(
    shared_structured_reference_rejects_wrong_kind,
    "bad-ref-kind"
);
rejected!(
    shared_structured_block_rejects_unknown_attribute,
    "bad-block-attr"
);
rejected!(
    shared_structured_field_rejects_unknown_attribute,
    "bad-field-attr"
);

#[test]
fn shared_structured_unknown_type_rejects_profile_with_bounds() {
    let source = case("bad-type");
    let error = load_profile_from_source(&source)
        .expect_err("unknown list member type must reject profile");
    let (start, end) = error
        .manifest_source_span()
        .expect("profile error must retain source bounds");
    assert!(start < end && end <= source.len());
    assert!(error.manifest_source_line().is_some());
}

#[test]
fn shared_structured_inline_uses_existing_semantic_registry() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let profile = load_profile_from_source(&case("profile")).unwrap();
    let document = parse_bsol_document(&case("valid")).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let mut registry = bsol::ValidatorRegistry::default();
    registry.on_rule("item", move |block| {
        assert!(matches!(
            block.value("name"),
            Some(ValidatedValue::String(_))
        ));
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    bsol::validate_with(&document, &profile, &registry).unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "inline and top-level item use the same registry"
    );
}

#[test]
fn recursive_inline_defaults_fail_closed_with_source_bounds() {
    let source = r#"profile "recursive.v2" {
      version = 2
      rule "item" {
        scope = top match = keyword keyword = item label = optional cardinality = one
        field "child" { type = inline(item) required = false default = item {} }
      }
    }"#;
    let profile = load_profile_from_source(source).unwrap();
    let document = parse_bsol_document("item {}").unwrap();
    let error =
        validate(&document, &profile).expect_err("recursive defaults must have bounded work/depth");
    assert!(
        error.to_string().contains("limit"),
        "must diagnose budget exhaustion: {error}"
    );
    let (start, end) = error
        .manifest_source_span()
        .expect("default diagnostics retain profile source bounds");
    assert!(start < end && end <= source.len());
}

#[test]
fn validation_work_budget_covers_all_fields_in_one_document() {
    let profile = load_profile_from_source(r#"profile "work.v2" {
      version = 2
      rule "config" {
        scope = top match = keyword keyword = config label = forbidden cardinality = one
        field "first" { type = list[u32] required = true }
        field "second" { type = list[u32] required = true }
      }
    }"#).unwrap();
    // Each field is individually below the work budget; their combined validation
    // must not reset it. The input stays below the 1 MiB subject/source ceiling.
    let values = std::iter::repeat_n("0", 60_000).collect::<Vec<_>>().join(",");
    let source = format!("config {{ first = [{values}] second = [{values}] }}");
    assert!(source.len() < 1_048_576);
    // 120000 list values exceed the default 100000-node syntax budget, so the caller raises it
    // explicitly to reach the validation work budget.
    assert!(parse_bsol_document(&source).is_err());
    let syntax = bsol::BsolLimits {
        max_nodes: 200_000,
        ..bsol::BsolLimits::default()
    };
    let document = bsol::parse_bsol_document_with_limits(&source, &syntax).unwrap();
    let error = validate(&document, &profile).expect_err("document-wide work must be bounded");
    assert!(error.to_string().contains("work limit"), "{error}");
    let (start, end) = error.manifest_source_span().unwrap();
    assert!(start < end && end <= source.len());
}

rejected!(
    shared_structured_absent_block_allowlist_is_closed,
    "bad-empty-block-attr"
);
rejected!(
    shared_structured_absent_field_allowlist_is_closed,
    "bad-empty-field-attr"
);
