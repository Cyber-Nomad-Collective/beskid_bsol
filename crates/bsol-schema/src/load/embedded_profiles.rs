use crate::{BsolError, SchemaProfile};

use super::document::load_profile_from_source;

pub(super) const EMBEDDED_PROFILES: &[(&str, &str)] = &[
    (
        "schema.v1",
        include_str!("../../../../schemas/schema.v1.bsol"),
    ),
    (
        "schema.v2",
        include_str!("../../../../schemas/schema.v2.bsol"),
    ),
    (
        "project.v1",
        include_str!("../../../../schemas/project.v1.bsol"),
    ),
    (
        "project.v2",
        include_str!("../../../../schemas/project.v2.bsol"),
    ),
    (
        "workspace.v1",
        include_str!("../../../../schemas/workspace.v1.bsol"),
    ),
    (
        "runtime.v1",
        include_str!("../../../../schemas/runtime.v1.bsol"),
    ),
    (
        "runtime.v2",
        include_str!("../../../../schemas/runtime.v2.bsol"),
    ),
    (
        "board.v1",
        include_str!("../../../../schemas/board.v1.bsol"),
    ),
    (
        "board.v2",
        include_str!("../../../../schemas/board.v2.bsol"),
    ),
    (
        "board.v3",
        include_str!("../../../../schemas/board.v3.bsol"),
    ),
    (
        "shell.pages.v1",
        include_str!("../../../../schemas/shell.pages.v1.bsol"),
    ),
    (
        "tools.config.v1",
        include_str!("../../../../schemas/tools.config.v1.bsol"),
    ),
    (
        "configuration.v1",
        include_str!("../../../../schemas/configuration.v1.bsol"),
    ),
    (
        "configuration.v2",
        include_str!("../../../../schemas/configuration.v2.bsol"),
    ),
];

/// Load an embedded schema profile by name (for example `project.v1`).
pub fn load_profile(name: &str) -> Result<SchemaProfile, BsolError> {
    let source = EMBEDDED_PROFILES
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, src)| *src)
        .ok_or_else(|| BsolError::UnknownProfile(name.to_string()))?;
    load_profile_from_source(source)
}
