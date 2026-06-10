# Getting started with BSOL

BSOL (Beskid Structured Object Language) describes structured configuration as nested blocks:

```bsol
myapp {
  name = "myapp"
  version = "1.0.0"
  root = "Src"
}
```

## Parse and validate

Add the facade crate:

```toml
[dependencies]
bsol = "0.1"
```

```rust
use bsol::{load_profile, parse_bsol_document, validate};

fn main() -> Result<(), bsol::BsolError> {
    let doc = parse_bsol_document(include_str!("example.bproj"))?;
    let profile = load_profile("project.v1")?;
    let _validated = validate(&doc, &profile)?;
    Ok(())
}
```

## Schema profiles

Profiles live under [`schemas/`](../schemas/) and declare allowed block shapes. Embedded profiles include:

- `project.v1` / `project.v2` — `.bproj` manifests (v2 adds variants, constraints, migration)
- `workspace.v1` — `.bws` workspaces
- `runtime.v1` / `runtime.v2` — runtime manifest BSOL (v2 uses rule inheritance)
- `board.v1` / `board.v2` / `board.v3` — shell layout boards (v3 uses cross-block refs)
- `configuration.v1` / `configuration.v2` — shared module and compiler configuration documents
- `schema.v1` / `schema.v2` — meta-schema for profile documents

## Custom validators

Register semantic checks after structural validation:

```rust
use bsol::{validate_with, ValidatorRegistry, load_profile, parse_bsol_document};

let mut registry = ValidatorRegistry::default();
registry.on_rule("node", |block| {
    if block.fields.get("kind") == Some(&"panel".to_string()) && block.fields.get("widget").is_none() {
        return Err(bsol::BsolError::Schema("panel nodes require widget".into()));
    }
    Ok(())
});
```

## Language server

```bash
cargo run -p bsol-lsp
```

Connects over stdio with diagnostics from parse + schema validation.

## Migration (v2)

```bash
beskid migrate-bsol --to project.v2 path/to/file.bproj
beskid validate-bsol --profile project.v2 --migrate path/to/file.bproj
```

See [v2 features](language/v2-features.md).

## Further reading

- [Syntax](language/syntax.md)
- [Schema profiles](language/schema-profiles.md)
- [Import schema](language/import-schema.md)
- [Generic blocks](language/generic-blocks.md)
- [Tree-sitter grammar](tree-sitter.md)
