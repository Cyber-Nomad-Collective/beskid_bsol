//! Import closure budgets agree with native `Core.Bsol.ImportGraph`: codes, messages and the
//! first violation in resolution order.
use std::collections::HashMap;
use std::path::Path;

use bsol_analysis::{
    resolve_active_profile, resolve_import_graph, ImportFailure, ImportGraphFailure,
    MaterializedSchema, SchemaSource,
};
use bsol_schema::{ImportSchemaSpec, ImportSource};
use bsol_syntax::{parse_bsol_document_counted, BsolLimits};

/// In-memory host: `file` path to source, identity `file:<path>`.
struct Host(HashMap<&'static str, String>);

impl SchemaSource for Host {
    fn materialize(
        &self,
        spec: &ImportSchemaSpec,
        _base_dir: &Path,
    ) -> Result<MaterializedSchema, ImportFailure> {
        let ImportSource::File { path } = &spec.from else {
            return Err(ImportFailure::new(
                "MissingCache",
                "fixture only materializes files",
            ));
        };
        let source = self.0.get(path.as_str()).ok_or_else(|| {
            ImportFailure::new("MissingCache", "fixture has no materialized path")
        })?;
        Ok(MaterializedSchema::owned(
            source.clone(),
            format!("file:{path}"),
        ))
    }
}

fn profile(name: &str, imports: &[(&str, &str)]) -> String {
    let imports = imports
        .iter()
        .map(|(profile, path)| {
            format!(" import_schema \"{profile}\" {{ from = file path = \"{path}\" }}")
        })
        .collect::<String>();
    format!("profile \"{name}\" {{ version = 2{imports} }}")
}

fn host(files: &[(&'static str, String)]) -> Host {
    Host(files.iter().cloned().collect())
}

fn root(source: &str) -> MaterializedSchema {
    MaterializedSchema::owned(source.to_string(), "file:root.bsol".into())
}

fn resolve(source: &str, host: &Host, limits: &BsolLimits) -> Result<usize, ImportGraphFailure> {
    resolve_import_graph(root(source), Path::new("."), host, limits)
        .map(|graph| graph.sources.len())
}

fn rejects(source: &str, host: &Host, limits: &BsolLimits, code: &str, message: &str) {
    let error = resolve(source, host, limits).expect_err(code);
    assert_eq!(
        (error.code.as_str(), error.message.as_str()),
        (code, message),
        "{error}"
    );
}

fn limits(edit: impl FnOnce(&mut BsolLimits)) -> BsolLimits {
    let mut limits = BsolLimits::default();
    edit(&mut limits);
    limits
}

fn nodes(source: &str) -> usize {
    parse_bsol_document_counted(source, &BsolLimits::default())
        .unwrap()
        .1
}

#[test]
fn defaults_match_the_native_descriptor() {
    let defaults = BsolLimits::default();
    assert_eq!(defaults.max_output_bytes, 8_388_608);
    assert_eq!(defaults.max_imports, 256);
    assert_eq!(defaults.max_import_depth, 32);
    assert_eq!(defaults.max_resolved_schema_bytes, 8_388_608);
}

#[test]
fn import_count_is_aggregate_over_the_closure() {
    let files = host(&[
        ("a.bsol", profile("a.v2", &[("c.v2", "c.bsol")])),
        ("b.bsol", profile("b.v2", &[])),
        ("c.bsol", profile("c.v2", &[])),
    ]);
    let source = profile("root.v2", &[("a.v2", "a.bsol"), ("b.v2", "b.bsol")]);
    assert_eq!(
        resolve(&source, &files, &limits(|l| l.max_imports = 3)),
        Ok(4)
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_imports = 2),
        "ImportCountLimit",
        "aggregate import count exceeded",
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_imports = 0),
        "ImportCountLimit",
        "aggregate import count exceeded",
    );
}

#[test]
fn import_depth_counts_nested_imports() {
    let files = host(&[
        ("a.bsol", profile("a.v2", &[("b.v2", "b.bsol")])),
        ("b.bsol", profile("b.v2", &[])),
    ]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    assert_eq!(
        resolve(&source, &files, &limits(|l| l.max_import_depth = 2)),
        Ok(3)
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_import_depth = 1),
        "ImportDepthLimit",
        "resolved import depth exceeded",
    );
}

#[test]
fn deep_chains_stop_at_the_default_depth() {
    let mut files = Vec::new();
    for index in 0..40 {
        let path: &'static str = Box::leak(format!("p{index}.bsol").into_boxed_str());
        let next = format!("p{}.v2", index + 1);
        let next_path = format!("p{}.bsol", index + 1);
        files.push((
            path,
            profile(&format!("p{index}.v2"), &[(&next, &next_path)]),
        ));
    }
    let files = host(&files);
    let source = profile("root.v2", &[("p0.v2", "p0.bsol")]);
    rejects(
        &source,
        &files,
        &BsolLimits::default(),
        "ImportDepthLimit",
        "resolved import depth exceeded",
    );
}

#[test]
fn resolved_schema_bytes_are_aggregate_and_checked_per_source() {
    let a = profile("a.v2", &[]);
    let files = host(&[("a.bsol", a.clone())]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    let total = source.len() + a.len();
    assert_eq!(
        resolve(
            &source,
            &files,
            &limits(|l| l.max_resolved_schema_bytes = total)
        ),
        Ok(2)
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_resolved_schema_bytes = total - 1),
        "ResolvedSchemaLimit",
        "aggregate schema bytes exceeded",
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_resolved_schema_bytes = source.len() - 1),
        "ResolvedSchemaLimit",
        "aggregate schema bytes exceeded",
    );
    // A host source larger than the whole budget fails its own verification before any visit.
    let large = format!("{}\n# {}", profile("a.v2", &[]), "x".repeat(200));
    let files = host(&[("a.bsol", large)]);
    rejects(
        &source,
        &files,
        &limits(|l| l.max_resolved_schema_bytes = source.len() + 10),
        "LimitExceeded",
        "materialized source exceeds the resolved schema budget",
    );
}

#[test]
fn diamonds_debit_one_source_once() {
    let base = profile("base.v2", &[]);
    let files = host(&[
        ("base.bsol", base.clone()),
        (
            "branch.bsol",
            profile("branch.v2", &[("base.v2", "base.bsol")]),
        ),
    ]);
    let source = "profile \"root.v2\" { version = 2 import_schema \"base.v2\" { from = file path = \"base.bsol\" alias = direct } import_schema \"branch.v2\" { from = file path = \"branch.bsol\" } }";
    let total = source.len() + base.len() + files.0["branch.bsol"].len();
    assert_eq!(
        resolve(
            source,
            &files,
            &limits(|l| l.max_resolved_schema_bytes = total)
        ),
        Ok(3)
    );
}

#[test]
fn aggregate_nodes_bound_each_import_parse() {
    let a = profile("a.v2", &[]);
    let files = host(&[("a.bsol", a.clone())]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    let total = nodes(&source) + nodes(&a);
    assert_eq!(
        resolve(&source, &files, &limits(|l| l.max_nodes = total)),
        Ok(2)
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_nodes = nodes(&source)),
        "ResolvedNodeLimit",
        "aggregate schema nodes exceeded",
    );
    rejects(
        &source,
        &files,
        &limits(|l| l.max_nodes = total - 1),
        "ImportSyntaxError",
        "syntax node limit",
    );
}

#[test]
fn first_violation_follows_native_resolution_order() {
    let files = host(&[("a.bsol", profile("a.v2", &[]))]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    // The declaration is counted before its source is resolved, which is before its depth.
    let both = limits(|l| {
        l.max_imports = 0;
        l.max_import_depth = 0;
    });
    rejects(
        &source,
        &files,
        &both,
        "ImportCountLimit",
        "aggregate import count exceeded",
    );
    let depth_only = limits(|l| l.max_import_depth = 0);
    rejects(
        &source,
        &files,
        &depth_only,
        "ImportDepthLimit",
        "resolved import depth exceeded",
    );
    // Depth precedes the aggregate byte budget of the visited source.
    let depth_and_bytes = limits(|l| {
        l.max_import_depth = 0;
        l.max_resolved_schema_bytes = source.len() + 1;
    });
    rejects(
        &source,
        &files,
        &depth_and_bytes,
        "ImportDepthLimit",
        "resolved import depth exceeded",
    );
}

#[test]
fn cycles_and_profile_identity_are_rejected() {
    let files = host(&[("root.bsol", profile("root.v2", &[("root.v2", "root.bsol")]))]);
    let source = profile("root.v2", &[("root.v2", "root.bsol")]);
    rejects(
        &source,
        &files,
        &BsolLimits::default(),
        "ImportCycle",
        "active canonical source imported recursively",
    );
    let files = host(&[("a.bsol", profile("other.v2", &[]))]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    rejects(
        &source,
        &files,
        &BsolLimits::default(),
        "ImportProfileMismatch",
        "materialization contains another profile",
    );
}

#[test]
fn active_profile_composition_uses_the_bounded_closure() {
    let files = host(&[(
        "a.bsol",
        "profile \"a.v2\" { version = 2 rule \"option\" { scope = top match = keyword keyword = option label = forbidden cardinality = many } }".to_string(),
    )]);
    let source = profile("root.v2", &[("a.v2", "a.bsol")]);
    let active = resolve_active_profile(
        root(&source),
        Path::new("."),
        &files,
        &BsolLimits::default(),
    )
    .expect("closure composes");
    assert!(active.rule("option").is_some());
    let error = resolve_active_profile(
        root(&source),
        Path::new("."),
        &files,
        &limits(|l| l.max_imports = 0),
    )
    .expect_err("closure budget applies to composition");
    assert!(
        error
            .to_string()
            .contains("ImportCountLimit: aggregate import count exceeded"),
        "{error}"
    );
}
