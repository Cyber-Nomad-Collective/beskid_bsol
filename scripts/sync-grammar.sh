#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GRAMMAR="$ROOT/grammars/tree-sitter-bsol"
PEST="$ROOT/crates/bsol-syntax/src/bsol.pest"

echo "BSOL grammar sync (pest manifest → tree-sitter keywords)"
rg -o 'ident|quoted_string|bracket_list' "$PEST" | sort -u > "$GRAMMAR/generated-keywords.txt" || true

if command -v tree-sitter >/dev/null 2>&1; then
  (cd "$GRAMMAR" && tree-sitter generate && tree-sitter test)
else
  echo "tree-sitter CLI not installed; skipped generate/test"
fi
