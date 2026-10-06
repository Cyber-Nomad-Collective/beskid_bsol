# Shared BSOL conformance inputs

These UTF-8 files are consumed by the Rust syntax integration tests and are intended for the future native corelib harness. Expected decoded scalars, canonical syntax and rejection policies live in `crates/bsol-syntax/tests/v06_grammar.rs` while the native harness and complete profile/typed corpus are still being implemented.

- `v06-values.bsol`: canonical @ attributes, escaped labels/scalars, exact numeric list items, empty map and nested RHS value blocks.
- `runtime-manifest-v5.bsol`: portable snapshot of the compiler ABI-v5 runtime manifest at the accepted `95c203ff` compiler baseline. It tests the full pre-existing scalar/list/map syntax and canonical semantic round trip without depending on a compiler checkout path.

These cases establish Rust bootstrap grammar coverage; they do not claim native execution, complete typed serialization or all-target corpus qualification.

`cases/schema-constraints/` defines the native/Rust BSOL-06-04 constraint, typed-default, conditional and variant contract. `expectations.json` records exact input/profile SHA256 identities, positive typed values and negative diagnostic categories. Native `BsolSchemaConstraintTests` reads those same inputs through `BESKID_BSOL_CORPUS`. These are newly authored RED cases: native implementation and the paired standalone semantic harness have not yet passed them. In particular, the existing Rust textual-default and conditional-map behavior needs repair; syntax parsing alone does not establish semantic conformance.

`cases/builtin-profiles/` contains paired ordinary valid documents and syntactically valid missing-root negatives for all fourteen embedded profiles. Its expectations ledger pins input and profile SHA256 identities. Native `BsolBuiltinProfilesTests` validates the shared documents and requires the Cardinality diagnostic for absent mandatory roots. These cases are authored but not yet native-qualified; profile validation does not establish generated typed adapter qualification.
