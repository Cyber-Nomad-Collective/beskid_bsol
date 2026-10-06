//! Rust and native corelib consume the same 0.6 schema cases.
use bsol::{ValidatedValue, load_profile_from_source, parse_bsol_document, validate};

fn case(name: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/schema-constraints");
    std::fs::read_to_string(root.join(format!("{name}.bsol"))).unwrap()
}

#[test]
fn shared_inline_conditions_retain_profile_metadata() {
    let profile = load_profile_from_source(&case("profile")).unwrap();
    let constraints = &profile.rules["config"].fields["url"].constraints;
    assert_eq!(constraints.required_if.get("source").map(String::as_str), Some("git"));
    assert_eq!(constraints.forbid_if.get("source").map(String::as_str), Some("path"));
}

#[test]
fn shared_defaults_retain_typed_values() {
    let profile = load_profile_from_source(&case("profile")).unwrap();
    let document = parse_bsol_document(&case("defaults")).unwrap();
    let result = validate(&document, &profile).unwrap();
    let block = &result.blocks[0];
    assert_eq!(block.value("count"), Some(&ValidatedValue::U32(3)));
    assert_eq!(
        block.value("name"),
        Some(&ValidatedValue::String("default-name".into()))
    );
    assert_eq!(
        block.value("rev"),
        Some(&ValidatedValue::String("head".into()))
    );
}

#[test]
fn shared_supplied_values_override_defaults_at_inclusive_maximum() {
    let profile = load_profile_from_source(&case("profile")).unwrap();
    let document = parse_bsol_document(&case("override")).unwrap();
    let result = validate(&document, &profile).unwrap();
    let block = &result.blocks[0];
    assert_eq!(block.value("count"), Some(&ValidatedValue::U32(5)));
    assert_eq!(
        block.value("name"),
        Some(&ValidatedValue::String("explicit".into()))
    );
    assert_eq!(
        block.value("rev"),
        Some(&ValidatedValue::String("exact".into()))
    );
}

macro_rules! rejected_case {
    ($name:ident, $case:literal) => {
        #[test]
        fn $name() {
            let profile = load_profile_from_source(&case("profile")).unwrap();
            let document = parse_bsol_document(&case($case)).unwrap();
            assert!(
                validate(&document, &profile).is_err(),
                "shared case must reject: {}",
                $case
            );
        }
    };
}

rejected_case!(shared_below_minimum, "below-min");
rejected_case!(shared_above_maximum, "above-max");
rejected_case!(shared_required_if_inline_map, "required-if");
rejected_case!(shared_forbid_if_inline_map, "forbid-if");
rejected_case!(shared_missing_variant_field, "variant-missing");
rejected_case!(shared_unknown_variant, "variant-unknown");

#[test]
fn shared_invalid_typed_default_rejects_profile() {
    assert!(load_profile_from_source(&case("invalid-default")).is_err());
}
