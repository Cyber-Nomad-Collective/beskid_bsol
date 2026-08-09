use super::{embedded_profiles::EMBEDDED_PROFILES, load_profile};

#[test]
fn load_project_profile() {
    let profile = load_profile("project.v1").expect("load project profile");
    assert_eq!(profile.name, "project.v1");
    assert!(profile.rule("root").is_some());
    assert!(profile.rule("target").is_some());
}

#[test]
fn load_schema_meta_profile() {
    let profile = load_profile("schema.v1").expect("load schema.v1");
    assert_eq!(profile.name, "schema.v1");
    assert!(profile.rule("profile").is_some());
}

#[test]
fn load_configuration_profile() {
    let profile = load_profile("configuration.v1").expect("load configuration.v1");
    assert_eq!(profile.name, "configuration.v1");
    assert!(profile.rule("config").is_some());
    assert!(profile.rule("moduleConfig").is_some());
    assert!(profile.rule("option").is_some());
}

#[test]
fn all_embedded_profiles_load() {
    for (name, _) in EMBEDDED_PROFILES {
        load_profile(name).unwrap_or_else(|e| panic!("load {name}: {e}"));
    }
}
