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

- `project.v1` — `.bproj` manifests
- `workspace.v1` — `.bws` workspaces
- `runtime.v1` — runtime manifest BSOL
- `board.v1` / `board.v2` — shell layout boards
- `schema.v1` — meta-schema for profile documents

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

## Further reading

- [Syntax](language/syntax.md)
- [Schema profiles](language/schema-profiles.md)
- [Import schema](language/import-schema.md)
- [Generic blocks](language/generic-blocks.md)
- [Tree-sitter grammar](tree-sitter.md)
