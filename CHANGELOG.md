# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Add shared valid and missing-root conformance documents for all fourteen builtin profiles, with pinned input identities and paired Rust/native validation gates.

- Add controlled offline schema materialization with opened directory capabilities, exact Git/registry/file locks, bounded owned UTF-8 reads and SHA-256 verification; remove unchecked package-cache resolution.

- Add a canonical semantic document/value writer and shared v0.6 grammar fixtures, including the pinned ABI-v5 runtime manifest.

- Commit the generated Tree-sitter BSOL parser, node types, grammar JSON, and
  runtime headers required by editor consumers that load the grammar directly.

### Changed

- Preserve numeric literals as explicit exact-spelling AST values and accept signed decimals, finite exponent values, numeric lists, empty maps, and inline blocks on assignment right-hand sides.
- Require canonical `@Name` attributes; replace bracket attribute examples and reject malformed argument separators.

- Replace the former MIT-or-Apache choice with a single Apache-2.0 license and
  expose that SPDX declaration through every workspace crate.

### Fixed

- Decode strict string escapes and Unicode surrogate pairs, preserve nested syntax during formatting, reject trailing malformed body tokens, and keep Unicode diagnostic spans on character boundaries with one-based lines.

- Align the editor Tree-sitter grammar and highlighting queries with the canonical BSOL surface,
  including attributes, scalar values, references, inline containers, type expressions, trailing
  commas, and balanced schemaless bodies; production schema fixtures now guard against drift.
- Restore sibling visibility required by the split BSOL parser and remove stale extracted imports.
- Resolve embedded schema profiles from their canonical repository paths after the loader split.
