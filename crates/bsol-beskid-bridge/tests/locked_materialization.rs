use bsol_beskid_bridge::{LockedImport, LockedSchemaHost, SchemaImportRequest};
use std::fs;

#[test]
fn bootstrap_adapter_rejects_wrong_declared_profile_before_composition() {
    let root = tempfile::tempdir().unwrap();
    let source = include_str!("../../../schemas/project.v1.bsol");
    fs::write(root.path().join("profile.bsol"), source).unwrap();
    let request = SchemaImportRequest::File {
        path: "profile.bsol".into(),
    };
    let lock = LockedImport::new(
        request,
        "project-profile".into(),
        "profile.bsol".into(),
        LockedSchemaHost::source_digest(source.as_bytes()),
    );
    let host = LockedSchemaHost::open(root.path(), vec![lock], 65536).unwrap();
    let mut spec = bsol_schema::ImportSchemaSpec {
        name: "project.v1".into(),
        alias: Some("project".into()),
        from: bsol_schema::ImportSource::File {
            path: "profile.bsol".into(),
        },
        span: Default::default(),
    };
    assert_eq!(
        bsol_analysis::SchemaSource::resolve(&host, &spec, root.path())
            .unwrap()
            .name,
        "project.v1"
    );
    spec.name = "foreign-profile".into();
    assert!(bsol_analysis::SchemaSource::resolve(&host, &spec, root.path()).is_err());
}

#[test]
fn locked_host_returns_owned_digest_verified_source_and_rejects_changed_bytes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("profile.bsol"), "schema \"demo\" {}\n").unwrap();
    let request = SchemaImportRequest::File {
        path: "profile.bsol".into(),
    };
    let lock = LockedImport::new(
        request.clone(),
        "workspace/demo".into(),
        "profile.bsol".into(),
        LockedSchemaHost::source_digest(b"schema \"demo\" {}\n"),
    );
    let host = LockedSchemaHost::open(root.path(), vec![lock], 1024).unwrap();
    let materialized = host.resolve(&request).unwrap();
    assert_eq!(materialized.source, "schema \"demo\" {}\n");
    assert_eq!(materialized.canonical_identity, "workspace/demo");
    fs::write(root.path().join("profile.bsol"), "schema \"changed\" {}\n").unwrap();
    assert!(host.resolve(&request).is_err());
    assert_eq!(materialized.source, "schema \"demo\" {}\n");
}

#[test]
fn mutable_git_and_registry_ranges_cannot_enter_locked_host() {
    let root = tempfile::tempdir().unwrap();
    for request in [
        SchemaImportRequest::Git {
            url: "https://example.org/schema.git".into(),
            revision: "main".into(),
            path: "profile.bsol".into(),
        },
        SchemaImportRequest::Registry {
            package: "demo".into(),
            version: "^1.0.0".into(),
            path: "profile.bsol".into(),
        },
        SchemaImportRequest::File {
            path: "../profile.bsol".into(),
        },
    ] {
        let lock = LockedImport::new(request, "demo".into(), "profile.bsol".into(), [0; 32]);
        assert!(LockedSchemaHost::open(root.path(), vec![lock], 1024).is_err());
    }
}

#[test]
fn git_registry_and_shorthand_share_exact_offline_locks() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("cache")).unwrap();
    fs::write(root.path().join("cache/profile.bsol"), "owned").unwrap();
    let git = SchemaImportRequest::Git {
        url: "https://example.org/schema.git".into(),
        revision: "a".repeat(40),
        path: "profile.bsol".into(),
    };
    let registry = SchemaImportRequest::Registry {
        package: "demo".into(),
        version: "1.2.3".into(),
        path: "profile.bsol".into(),
    };
    let digest = LockedSchemaHost::source_digest(b"owned");
    let host = LockedSchemaHost::open(
        root.path(),
        vec![
            LockedImport::new(
                git.clone(),
                "git/demo".into(),
                "cache/profile.bsol".into(),
                digest,
            ),
            LockedImport::new(
                registry.clone(),
                "registry/demo/1.2.3".into(),
                "cache/profile.bsol".into(),
                digest,
            ),
        ],
        5,
    )
    .unwrap();
    assert_eq!(host.resolve(&git).unwrap().source, "owned");
    assert_eq!(host.resolve(&registry).unwrap().source, "owned");
    let shorthand = SchemaImportRequest::pckg("@pckg/demo/1.2.3/profile.bsol").unwrap();
    assert_eq!(
        host.resolve(&shorthand).unwrap().canonical_identity,
        "registry/demo/1.2.3"
    );
    assert!(host
        .resolve(&SchemaImportRequest::File {
            path: "cache/profile.bsol".into()
        })
        .is_err());
}

#[test]
fn limits_utf8_duplicate_locks_and_identity_conflicts_reject() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("profile.bsol"), [255_u8]).unwrap();
    let request = SchemaImportRequest::File {
        path: "profile.bsol".into(),
    };
    let lock = LockedImport::new(
        request.clone(),
        "demo".into(),
        "profile.bsol".into(),
        LockedSchemaHost::source_digest(&[255]),
    );
    assert!(LockedSchemaHost::open(root.path(), vec![lock.clone(), lock.clone()], 1024).is_err());
    let host = LockedSchemaHost::open(root.path(), vec![lock.clone()], 1024).unwrap();
    assert!(host.resolve(&request).is_err());
    fs::write(root.path().join("profile.bsol"), "oversized").unwrap();
    let limited = LockedSchemaHost::open(root.path(), vec![lock.clone()], 2).unwrap();
    assert!(limited.resolve(&request).is_err());
    let conflicting = LockedImport::new(
        SchemaImportRequest::File {
            path: "other.bsol".into(),
        },
        "demo".into(),
        "other.bsol".into(),
        [0; 32],
    );
    assert!(LockedSchemaHost::open(root.path(), vec![lock, conflicting], 1024).is_err());
}

#[test]
fn explicit_lock_configuration_rejects_unknown_fields_and_short_digests() {
    let prefix = r#"{"request":{"source":"file","path":"profile.bsol"},"canonical_identity":"demo","materialized_path":"profile.bsol","source_sha256":["#;
    let bytes = std::iter::repeat_n("0", 32).collect::<Vec<_>>().join(",");
    let valid = format!("{prefix}{bytes}]}}");
    assert!(serde_json::from_str::<LockedImport>(&valid).is_ok());
    let unknown = valid.replace(
        "\"source\":\"file\"",
        "\"source\":\"file\",\"allow_escape\":true",
    );
    assert!(serde_json::from_str::<LockedImport>(&unknown).is_err());
    assert!(serde_json::from_str::<LockedImport>(&format!("{prefix}0]}}")).is_err());
}

#[cfg(unix)]
#[test]
fn contained_symlink_can_materialize_the_locked_content() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("owned.bsol"), "owned").unwrap();
    std::os::unix::fs::symlink("owned.bsol", root.path().join("profile.bsol")).unwrap();
    let request = SchemaImportRequest::File {
        path: "profile.bsol".into(),
    };
    let lock = LockedImport::new(
        request.clone(),
        "demo".into(),
        "profile.bsol".into(),
        LockedSchemaHost::source_digest(b"owned"),
    );
    let host = LockedSchemaHost::open(root.path(), vec![lock], 5).unwrap();
    assert_eq!(host.resolve(&request).unwrap().source, "owned");
}

#[cfg(unix)]
#[test]
fn symlink_escape_is_rejected_by_directory_capability() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("profile.bsol"), "secret").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("profile.bsol"),
        root.path().join("profile.bsol"),
    )
    .unwrap();
    let request = SchemaImportRequest::File {
        path: "profile.bsol".into(),
    };
    let lock = LockedImport::new(
        request.clone(),
        "demo".into(),
        "profile.bsol".into(),
        LockedSchemaHost::source_digest(b"secret"),
    );
    let host = LockedSchemaHost::open(root.path(), vec![lock], 1024).unwrap();
    assert!(host.resolve(&request).is_err());
}
