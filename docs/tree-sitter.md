# Tree-sitter grammar

The BSOL tree-sitter grammar lives in [`grammars/tree-sitter-bsol/`](../grammars/tree-sitter-bsol/).

## Canonical source

Pest grammar: [`crates/bsol-syntax/src/bsol.pest`](../crates/bsol-syntax/src/bsol.pest)

## Sync

```bash
./scripts/sync-grammar.sh
```

This regenerates the committed Tree-sitter parser, runs its corpus, and verifies every canonical
schema fixture parses without recovery nodes. The Pest grammar remains the exact validation
authority; Tree-sitter is its error-tolerant structural editor projection.

## Corpus tests

Fixtures under `grammars/tree-sitter-bsol/test/corpus/` cover blocks, attributes, scalar values,
references, inline maps and blocks, type expressions, trailing commas, and balanced
`@schemaless` bodies. The sync gate additionally parses every file under `schemas/`.

## Editor integration

Highlight queries: `grammars/tree-sitter-bsol/queries/highlights.scm`

Future: publish npm package `@cyber-nomad-collective/bsol-tree-sitter` (mirroring `beskid_treesitter`).
