# BSOL syntax

BSOL uses a generic **block model**: every construct is a block with a kind, optional label, and body.

## Blocks

```bsol
kind "label" {
  key = value
  nested {
    ...
  }
}
```

- **kind** — identifier (`target`, `dependency`, `profile`, or a free-form root name)
- **label** — optional quoted string (required for some schema rules)
- **body** — assignments and nested blocks

## Assignments

```bsol
name = "myapp"
kind = Lib
tags = [stable, default]
```

Value forms:

| Form | Example |
|------|---------|
| Quoted string | `"hello"` |
| Identifier / bare token | `Lib`, `8080` |
| Bracket list | `[a, b, "c"]` |

## Comments

```bsol
// line comment
# hash comment
```

## Schemaless escape hatch

When a block body is not BSOL (or is validated elsewhere), use `@schemaless`:

```bsol
payload @schemaless {
  arbitrary { text } not = parsed
}
```

The inner text is captured verbatim.

## Documents

A document is a sequence of top-level blocks. Schema profiles define which kinds are allowed and their cardinality.

See also: [schema profiles](schema-profiles.md), [generic blocks](generic-blocks.md).
