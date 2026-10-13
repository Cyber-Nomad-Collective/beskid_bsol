# Schema profiles

Schema profiles are BSOL documents describing allowed block shapes.

## Profile document

```bsol
profile "project.v1" {
  version = 1
  rule "root" {
    scope = top
    match = free_ident
    except = [target, dependency]
    label = forbidden
    cardinality = one
    field "name" { type = quoted required = true }
    nested "mod" { ... }
  }
  rule "target" { ... }
}
```

`version` selects the schema format: `1` or `2`. A profile loaded on its own defaults to `1`. Every profile in an import closure must declare exactly one supported `version`; the resolver rejects it otherwise with `InvalidProfileVersion`.

## Rule matchers

| `match` | Meaning |
|---------|---------|
| `keyword` + `keyword = foo` | Exact block kind `foo` |
| `keywords` + `keywords = [mod, meta]` | Any listed kind |
| `free_ident` + `except = [...]` | Any identifier not in except list |

## Field types

| Type | Validates |
|------|-----------|
| `quoted` | Double-quoted string |
| `ident` | Identifier (quoted strings accepted) |
| `u32` | Non-negative integer literal |
| `list` | Bracket list |
| `loose` | Quoted string or identifier |
| `enum_or_quoted` | Value in `values = [...]` or any quoted string |

## Cardinality

- `one` — exactly one block
- `many` — zero or more
- `zero_or_one` — optional, at most one

## Flags

- `extras = true` — allow unknown assignment keys
- `nested_extras = true` — allow nested blocks not declared in schema
- `schemaless = true` — block must use `@schemaless`

## Meta-schema

Profile documents themselves validate against embedded `schema.v1`.
