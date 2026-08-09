use super::validate;
use bsol_schema::load_profile;

#[test]
fn validates_project_v2_dependency_variants() {
    let src = r#"demo {
  name = "demo"
  version = "1.0.0"
  root = "."
}
dependency "core" {
  source = git
  url = "https://example.com"
  rev = "main"
}
"#;
    let doc = bsol_syntax::parse_bsol_document(src).expect("parse");
    let profile = load_profile("project.v2").expect("profile");
    validate(&doc, &profile).expect("validate git dependency");
}

#[test]
fn rejects_git_dependency_missing_rev() {
    let src = r#"demo {
  name = "demo"
  version = "1.0.0"
  root = "."
}
dependency "core" {
  source = git
  url = "https://example.com"
}
"#;
    let doc = bsol_syntax::parse_bsol_document(src).expect("parse");
    let profile = load_profile("project.v2").expect("profile");
    assert!(validate(&doc, &profile).is_err());
}
