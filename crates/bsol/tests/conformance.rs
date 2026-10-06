//! Conformance tests across embedded schema profiles.

use bsol::{
    load_profile, migrate_document, parse_bsol_document, validate, validate_profile_document,
};

#[test]
fn meta_schema_validates_itself() {
    let source = include_str!("../../../schemas/schema.v1.bsol");
    validate_profile_document(source).expect("schema.v1 self-validation");
}

#[test]
fn meta_schema_v2_validates() {
    let source = include_str!("../../../schemas/schema.v2.bsol");
    let profile = load_profile("schema.v2").expect("load schema.v2");
    let doc = parse_bsol_document(source).expect("parse");
    validate(&doc, &profile).expect("validate schema.v2 document");
}

#[test]
fn all_embedded_profiles_are_well_formed() {
    for name in [
        "project.v1",
        "project.v2",
        "workspace.v1",
        "runtime.v1",
        "runtime.v2",
        "board.v1",
        "board.v2",
        "board.v3",
        "shell.pages.v1",
        "configuration.v1",
        "configuration.v2",
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
fn board_v3_accepts_ref_root() {
    let src = r#"board "default" {
  version = 3
  root = @node/main
}
node "main" {
  kind = panel
  widget = "welcome"
}
"#;
    let doc = parse_bsol_document(src).expect("parse");
    let profile = load_profile("board.v3").expect("profile");
    validate(&doc, &profile).expect("validate");
}

#[test]
fn project_v2_migrates_git_dependency() {
    let src = r#"demo {
  name = "demo"
  version = "0.1.0"
  root = "."
}
dependency "core" {
  source = git
  url = "https://example.com"
}
"#;
    let (migrated, validated) = migrate_document(src, "project.v2").expect("migrate");
    assert!(migrated.contains("rev ="));
    assert_eq!(validated.profile, "project.v2");
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

const GLUE_PROJECT_HEADER: &str = r#"demo {
  name = "demo"
  version = "0.1.0"
}
target "lib" {
  kind = Lib
}
"#;

fn validate_glue_project(profile_name: &str, glue: &str) -> Result<bsol::ValidatedDocument, bsol::BsolError> {
    let src = format!("{GLUE_PROJECT_HEADER}{glue}");
    let doc = parse_bsol_document(&src).expect("parse");
    let profile = load_profile(profile_name).expect("profile");
    validate(&doc, &profile)
}

#[test]
fn project_profiles_accept_glue_owner_blocks() {
    for profile in ["project.v1", "project.v2"] {
        let validated = validate_glue_project(
            profile,
            "glue \"glue_manual\" {\n  backend = rust\n  path = \"rust\"\n}\nglue \"other\" {\n  backend = rust\n  path = \"other\"\n}\n",
        )
        .unwrap_or_else(|e| panic!("{profile}: {e}"));
        let glue: Vec<_> = validated.blocks.iter().filter(|block| block.rule_id == "glue").collect();
        assert_eq!(glue.len(), 2, "{profile}");
        assert_eq!(glue[0].label.as_deref(), Some("glue_manual"));
        assert_eq!(glue[0].fields.get("backend").map(String::as_str), Some("rust"));
        assert_eq!(glue[0].fields.get("path").map(String::as_str), Some("rust"));
    }
}

#[test]
fn project_profiles_reject_malformed_glue_owner_blocks() {
    for profile in ["project.v1", "project.v2"] {
        for glue in [
            // unknown key (tool paths are not manifest keys)
            "glue \"g\" {\n  backend = rust\n  path = \"rust\"\n  cargo = \"/usr/bin/cargo\"\n}\n",
            // missing label
            "glue {\n  backend = rust\n  path = \"rust\"\n}\n",
            // missing backend
            "glue \"g\" {\n  path = \"rust\"\n}\n",
            // missing path
            "glue \"g\" {\n  backend = rust\n}\n",
            // backend outside the closed enum
            "glue \"g\" {\n  backend = python\n  path = \"rust\"\n}\n",
            // unquoted path
            "glue \"g\" {\n  backend = rust\n  path = rust\n}\n",
        ] {
            assert!(validate_glue_project(profile, glue).is_err(), "{profile} accepted:\n{glue}");
        }
    }
}
