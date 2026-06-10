//! Resolved schema collection from profile imports.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bsol_schema::{
    BsolError, ImportSchemaSpec, ImportSource, SchemaProfile, load_profile_from_path,
    load_profile_from_source,
};

/// Trait for resolving imported schema profiles from external sources.
pub trait SchemaSource: Send + Sync {
    fn resolve(&self, spec: &ImportSchemaSpec, base_dir: &Path) -> Result<SchemaProfile, BsolError>;
}

/// Resolves schemas from the local filesystem only.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileSchemaSource;

impl SchemaSource for FileSchemaSource {
    fn resolve(&self, spec: &ImportSchemaSpec, base_dir: &Path) -> Result<SchemaProfile, BsolError> {
        match &spec.from {
            ImportSource::File { path } => {
                let resolved = base_dir.join(path);
                load_profile_from_path(&resolved)
            }
            other => Err(BsolError::Import(format!(
                "file resolver cannot resolve `{other:?}` for `{}`",
                spec.name
            ))),
        }
    }
}

/// Aggregated schema profiles keyed by logical name.
#[derive(Debug, Default, Clone)]
pub struct SchemaCollection {
    profiles: HashMap<String, SchemaProfile>,
}

impl SchemaCollection {
    pub fn insert(&mut self, name: String, profile: SchemaProfile) -> Result<(), BsolError> {
        if self.profiles.contains_key(&name) {
            return Err(BsolError::Import(format!(
                "schema profile name collision: `{name}`"
            )));
        }
        self.profiles.insert(name, profile);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&SchemaProfile> {
        self.profiles.get(name)
    }

    pub fn profiles(&self) -> impl Iterator<Item = (&String, &SchemaProfile)> {
        self.profiles.iter()
    }
}

/// Resolve a profile and all `import_schema` dependencies.
pub fn resolve_profile(
    profile: SchemaProfile,
    base_dir: &Path,
    source: &dyn SchemaSource,
) -> Result<SchemaCollection, BsolError> {
    let mut collection = SchemaCollection::default();
    resolve_profile_into(&mut collection, profile, base_dir, source)?;
    Ok(collection)
}

/// Compose the active validation profile from imports, extends, and overlays.
pub fn resolve_active_profile(
    profile: SchemaProfile,
    base_dir: &Path,
    source: &dyn SchemaSource,
) -> Result<SchemaProfile, BsolError> {
    let collection = resolve_profile(profile.clone(), base_dir, source)?;
    let mut active = collection
        .get(&profile.name)
        .cloned()
        .unwrap_or(profile);

    for import in &active.imports.clone() {
        let key = import.alias.clone().unwrap_or_else(|| import.name.clone());
        if let Some(imported) = collection.get(&key) {
            active = bsol_schema::merge_profiles(imported.clone(), active);
        } else if let Ok(imported) = source.resolve(import, base_dir) {
            active = bsol_schema::merge_profiles(imported, active);
        }
    }

    for extend in &active.extends.clone() {
        if let Some(base) = collection.get(&extend.base) {
            active = bsol_schema::merge_profiles(base.clone(), active);
        } else if let Ok(base) = load_profile_from_source_by_name(&extend.base) {
            let mut overlay = active.clone();
            overlay.extends = vec![extend.clone()];
            active = bsol_schema::merge_profiles(base, overlay);
        }
    }

    bsol_schema::compose_profile(active)
}

fn load_profile_from_source_by_name(name: &str) -> Result<SchemaProfile, BsolError> {
    bsol_schema::load_profile(name)
}

fn resolve_profile_into(
    collection: &mut SchemaCollection,
    profile: SchemaProfile,
    base_dir: &Path,
    source: &dyn SchemaSource,
) -> Result<(), BsolError> {
    for import in profile.imports.clone() {
        let key = import.alias.clone().unwrap_or_else(|| import.name.clone());
        if collection.get(&key).is_some() {
            continue;
        }
        let imported = source.resolve(&import, base_dir)?;
        resolve_profile_into(collection, imported, base_dir, source)?;
    }
    collection.insert(profile.name.clone(), profile)?;
    Ok(())
}

/// Composite resolver chaining multiple sources.
pub struct CompositeSchemaSource {
    sources: Vec<Box<dyn SchemaSource>>,
}

impl CompositeSchemaSource {
    pub fn new(sources: Vec<Box<dyn SchemaSource>>) -> Self {
        Self { sources }
    }

    pub fn file_only() -> Self {
        Self::new(vec![Box::new(FileSchemaSource)])
    }

    pub fn add_source(&mut self, source: Box<dyn SchemaSource>) {
        self.sources.push(source);
    }
}

impl SchemaSource for CompositeSchemaSource {
    fn resolve(&self, spec: &ImportSchemaSpec, base_dir: &Path) -> Result<SchemaProfile, BsolError> {
        let mut last_err = None;
        for source in &self.sources {
            match source.resolve(spec, base_dir) {
                Ok(profile) => return Ok(profile),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            BsolError::Import(format!("no resolver for schema `{}`", spec.name))
        }))
    }
}

/// Parse `@pckg/package/version/path` shorthand into registry import fields.
pub fn parse_pckg_shorthand(reference: &str) -> Result<(String, String, String), BsolError> {
    let rest = reference
        .strip_prefix("@pckg/")
        .ok_or_else(|| BsolError::Import(format!("invalid pckg reference `{reference}`")))?;
    let mut parts = rest.splitn(3, '/');
    let package = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing package".into()))?
        .to_string();
    let version = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing version".into()))?
        .to_string();
    let path = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing profile path".into()))?
        .to_string();
    Ok((package, version, path))
}

/// Load profile text from a resolved path (used by bridge implementations).
pub fn load_profile_text(path: &Path) -> Result<String, BsolError> {
    std::fs::read_to_string(path).map_err(|e| BsolError::Import(format!("read `{}`: {e}", path.display())))
}

/// Load profile from text after external fetch.
pub fn load_fetched_profile(source: &str) -> Result<SchemaProfile, BsolError> {
    load_profile_from_source(source)
}

/// Base directory helper for profile documents on disk.
pub fn profile_base_dir(path: Option<&Path>) -> PathBuf {
    path.and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pckg_reference() {
        let (pkg, ver, path) =
            parse_pckg_shorthand("@pckg/beskid-schemas/1.0.0/project.v1.bsol").expect("parse");
        assert_eq!(pkg, "beskid-schemas");
        assert_eq!(ver, "1.0.0");
        assert_eq!(path, "project.v1.bsol");
    }
}
