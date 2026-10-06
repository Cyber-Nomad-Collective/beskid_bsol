# BSOL — Beskid Structured Object Language

Block-based configuration language used for Beskid manifests (`.bproj`, `.bws`), runtime manifests, and shell layouts.

## Licensing

BSOL and its Beskid-owned crates are Apache-2.0. See
[LICENSING.md](LICENSING.md) for scope and redistribution guidance.

## Crates

| Crate | Role |
|-------|------|
| [`bsol-syntax`](crates/bsol-syntax) | Parser and generic block AST (no Beskid deps) |
| [`bsol-schema`](crates/bsol-schema) | Schema profile models and loader |
| [`bsol-pipeline`](crates/bsol-pipeline) | Analysis phase IDs and observers |
| [`bsol-analysis`](crates/bsol-analysis) | Validation, schema import resolution, session |
| [`bsol`](crates/bsol) | Facade crate for applications |
| [`bsol-lsp`](crates/bsol-lsp) | Language server binary |
| [`bsol-beskid-bridge`](crates/bsol-beskid-bridge) | Controlled offline file/Git/registry import bytes with exact content locks |

Schema imports require an explicitly configured host. `LockedSchemaHost` opens a
controlled materialization root and accepts a lock inventory; it never fetches
missing artifacts. Each lock contains `request` (tagged by `source`: `file`,
`git`, or `registry`), `canonical_identity`, `materialized_path`, and
`source_sha256` (an array of exactly 32 byte integers). Git requests include
`url`, full immutable `revision`, and `path`; registry requests include
`package`, exact SemVer `version`, and `path`. Package shorthand normalizes to
the same registry request. All paths are portable root-relative paths.

CLI BSOL validation accepts paired `--schema-import-root ROOT` and
`--schema-import-lock LOCK.json` options. The lock file is a JSON array of these
entries, capped at 1 MiB and 4096 entries. Reads use the opened directory
capability, reject escaping symlinks and changed digests, and return owned UTF-8
source. The bootstrap adapter parses those verified bytes with Rust tooling;
native consumers use the same owned-byte host contract with Beskid corelib.
An analysis session has no implicit filesystem import resolver.

## Quick start

```rust
use bsol::{load_profile, parse_bsol_document, validate};

let source = r#"demo {
  name = "demo"
  version = "0.1.0"
  root = "."
}"#;
let doc = parse_bsol_document(source)?;
let profile = load_profile("project.v1")?;
let validated = validate(&doc, &profile)?;
```

## CLI (via Beskid compiler)

```bash
beskid validate-bsol --profile project.v1 path/to/file.bproj
```

## Documentation

See [`docs/getting-started.md`](docs/getting-started.md) and [`docs/language/`](docs/language/).

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Publishing

Crates publish to [crates.io](https://crates.io) on GitHub release tags via [`.github/workflows/publish.yml`](.github/workflows/publish.yml).
