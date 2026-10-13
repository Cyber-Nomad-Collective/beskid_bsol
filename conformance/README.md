# Shared BSOL conformance inputs

This directory is the shared BSOL conformance corpus root. The Rust tests in
this repository and the native Corelib tests read the same UTF-8 files.

## Corpus root convention

`BESKID_BSOL_CORPUS` names this directory (`beskid_bsol/conformance`), not
`cases/`. Native tests append `cases/<set>/...` to read inputs and
`../schemas/...` to read the embedded profiles in `beskid_bsol/schemas`.
Release-case bindings and the release evidence producer use the same root.

## Case sets and consumers

The native consumers are targets of
`compiler/corelib/beskid_corelib/tests/corelib_tests/corelib_tests.bproj`.

| Input | Rust consumer | Native target |
| --- | --- | --- |
| `cases/v06-values.bsol` | `crates/bsol-syntax/tests/v06_grammar.rs` | `BsolSuite` |
| `cases/runtime-manifest-v5.bsol` | `crates/bsol-syntax/tests/v06_grammar.rs` | `BsolSuite` |
| `cases/builtin-profiles/` | `crates/bsol-analysis/tests/builtin_profiles_v06.rs` | `BsolSchemaTests` |
| `cases/schema-structured/` | `crates/bsol/tests/v06_schema_structured.rs` | `BsolSchemaRulesSuite` |
| `cases/schema-constraints/` | `crates/bsol/tests/v06_schema_constraints.rs` | `BsolSchemaRulesSuite` |
| `cases/schema-patterns/` | `crates/bsol/tests/v06_schema_patterns.rs` | `BsolSchemaRulesSuite` |
| `cases/schema-composition-references/` | none | `BsolSchemaRulesSuite` |
| `cases/schema-migration/` | none | `BsolMigrationTests` |
| `cases/serialization-v1/` | none | `BsolSerializationWireTests` |

- `v06-values.bsol`: canonical `@` attributes, escaped labels and scalars,
  exact numeric list items, an empty map and nested RHS value blocks.
- `runtime-manifest-v5.bsol`: a portable snapshot of the compiler ABI-v5
  runtime manifest. It tests the full scalar, list and map syntax and the
  canonical round trip without a compiler checkout path.
- `builtin-profiles/`: paired valid documents and missing-root negatives for
  all fourteen embedded profiles. The negatives require the Cardinality
  diagnostic.
- `schema-*/` sets: `expectations.json` ledgers pin input and profile SHA-256
  identities, positive typed values and negative diagnostic categories.
- `serialization-v1/`: typed `serialization.v1` wire documents.

The typed serialization targets `SerializationSuite`,
`SerializationModEligibilitySuite` and `SerializationGeneratedSuite` do not
read this corpus. They cover the format-neutral serialization library, the
Serialization Mod eligibility rules and the generated adapters.
