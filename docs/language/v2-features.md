# BSOL v2 language features

BSOL v2 extends the block model with richer types, composition, and migration.

## Syntax additions

- **Attributes:** `[Deprecated(since = "2.0")]` before blocks
- **References:** `@node/main_panel` cross-block links
- **Inline maps:** `env = { DEBUG = "1", PORT = 8080 }`
- **Typed lists:** `[a, @node/x, node { kind = panel }]`
- **Booleans:** `enabled = true`

## Schema profile v2

Profiles declare `version = 2` and may use:

- Parameterized field types: `list[ident]`, `ref(node)`, `map[quoted, loose]`
- Rule inheritance: `extends = dispatch_base`, `mixes = [trait]`
- Discriminated unions: `variant "git" { require = [url, rev] }`
- Field constraints: `default`, `min`, `max`, `pattern`, `required_if`
- Profile extension: `extend "project.v1" { rule "root" { ... } }`
- Migration routes: `migration { from = "project.v1" ... }`

## Migration CLI

```bash
beskid migrate-bsol --to project.v2 path.bproj
beskid validate-bsol --profile project.v2 --migrate path.bproj
```

See also: [getting-started.md](getting-started.md), platform spec BSOL design model.
