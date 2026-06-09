# Generic block model

BSOL extensibility rests on three layers:

## 1. Syntax — generic AST

Every block parses to:

```rust
BsolBlock {
    kind: String,
    label: Option<QuotedString>,
    items: Vec<BsolItem>,  // assignments | nested blocks
}
```

Unknown block kinds are syntax-valid; schema validation rejects or accepts them.

## 2. Schema — declarative rules

`rule` blocks define matchers and field contracts. Forward-compatible manifests use `extras = true` on root blocks.

Nested block rules reuse the same `BlockRule` structure recursively.

## 3. Semantic — custom validators

Applications register validators on rule ids:

```rust
registry.on_rule("node", |block| { /* cross-ref checks */ Ok(()) });
```

Board layout cross-references (`root` must name an existing `node`) belong here rather than in the core validator.

## Schemaless payloads

For foreign syntax inside BSOL documents, `@schemaless` preserves raw text for downstream processors.

## Adding a new document type

1. Author a profile under `schemas/`.
2. Validate the profile against `schema.v1`.
3. Optionally register semantic validators.
4. Lower `ValidatedDocument` to your domain model (as `beskid_analysis` does for `.bproj`).
