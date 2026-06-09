# BSOL — Beskid Structured Object Language

Block-based configuration language used for Beskid manifests (`.bproj`, `.bws`), runtime manifests, and shell layouts.

## Crates

| Crate | Role |
|-------|------|
| [`bsol-syntax`](crates/bsol-syntax) | Parser and generic block AST (no Beskid deps) |
| [`bsol-schema`](crates/bsol-schema) | Schema profile models and loader |
| [`bsol-pipeline`](crates/bsol-pipeline) | Analysis phase IDs and observers |
| [`bsol-analysis`](crates/bsol-analysis) | Validation, schema import resolution, session |
| [`bsol`](crates/bsol) | Facade crate for applications |
| [`bsol-lsp`](crates/bsol-lsp) | Language server binary |
| [`bsol-beskid-bridge`](crates/bsol-beskid-bridge) | Optional `@pckg/` import resolution |

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
