//! Shared ordinary documents complement native Corelib validation.
use std::{fs, path::PathBuf};
use bsol_analysis::validate;
use bsol_schema::load_profile;
use bsol_syntax::parse_bsol_document;

#[test]
fn all_fourteen_builtin_profiles_validate_shared_document_pairs() {
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/builtin-profiles");
    let names = ["schema.v1", "schema.v2", "project.v1", "project.v2", "workspace.v1",
        "runtime.v1", "runtime.v2", "board.v1", "board.v2", "board.v3", "shell.pages.v1",
        "tools.config.v1", "configuration.v1", "configuration.v2"];
    for name in names {
        let profile = load_profile(name).unwrap_or_else(|e| panic!("{name} profile: {e}"));
        let valid = parse_bsol_document(&fs::read_to_string(corpus.join(format!("{name}.valid.bsol"))).unwrap()).unwrap();
        validate(&valid, &profile).unwrap_or_else(|e| panic!("{name} valid document: {e}"));
        let invalid = parse_bsol_document(&fs::read_to_string(corpus.join(format!("{name}.invalid.bsol"))).unwrap()).unwrap();
        assert!(validate(&invalid, &profile).is_err(), "{name}: absent required root must reject");
    }
}
