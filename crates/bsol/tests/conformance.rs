//! Conformance tests across embedded schema profiles.

use bsol::{load_profile, parse_bsol_document, validate, validate_profile_document};

#[test]
fn meta_schema_validates_itself() {
    let source = include_str!("../../../schemas/schema.v1.bsol");
    validate_profile_document(source).expect("schema.v1 self-validation");
}

#[test]
fn all_embedded_profiles_are_well_formed() {
    for name in [
        "project.v1",
        "workspace.v1",
        "runtime.v1",
        "board.v1",
        "board.v2",
    ] {
        let profile = load_profile(name).unwrap_or_else(|e| panic!("load {name}: {e}"));
        assert_eq!(profile.name, name);
    }
}

#[test]
fn project_v1_accepts_minimal_host_manifest() {
    let src = r#"demo {
  name = "demo"
  version = "0.1.0"
  root = "."
}
target "app" {
  kind = App
  entry = "Main.bd"
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    let profile = load_profile("project.v1").expect("profile");
    validate(&doc, &profile).expect("validate");
}

#[test]
fn board_v2_accepts_minimal_layout() {
    let src = r#"board "default" {
  version = 2
  root = "main"
}
node "main" {
  kind = panel
  widget = "welcome"
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    let profile = load_profile("board.v2").expect("profile");
    validate(&doc, &profile).expect("validate");
}

#[test]
fn duplicate_field_keys_are_rejected() {
    let src = r#"demo {
  name = "a"
  name = "b"
  version = "1"
  root = "."
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    let profile = load_profile("project.v1").expect("profile");
    assert!(validate(&doc, &profile).is_err());
}
