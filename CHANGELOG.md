# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- Replace the former MIT-or-Apache choice with a single Apache-2.0 license and
  expose that SPDX declaration through every workspace crate.

### Fixed

- Restore sibling visibility required by the split BSOL parser and remove stale extracted imports.
- Resolve embedded schema profiles from their canonical repository paths after the loader split.
