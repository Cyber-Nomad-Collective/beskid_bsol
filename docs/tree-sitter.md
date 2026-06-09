# Tree-sitter grammar

The BSOL tree-sitter grammar lives in [`grammars/tree-sitter-bsol/`](../grammars/tree-sitter-bsol/).

## Canonical source

Pest grammar: [`crates/bsol-syntax/src/bsol.pest`](../crates/bsol-syntax/src/bsol.pest)

## Sync

```bash
./scripts/sync-grammar.sh
```

This regenerates `grammar.js` keywords from pest and runs `tree-sitter test`.

## Corpus tests

Fixtures under `grammars/tree-sitter-bsol/test/corpus/` cover blocks, assignments, lists, and `@schemaless`.

## Editor integration

Highlight queries: `grammars/tree-sitter-bsol/queries/highlights.scm`

Future: publish npm package `@cyber-nomad-collective/bsol-tree-sitter` (mirroring `beskid_treesitter`).
