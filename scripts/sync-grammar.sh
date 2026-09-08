#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GRAMMAR="$ROOT/grammars/tree-sitter-bsol"

if command -v tree-sitter >/dev/null 2>&1; then
  tree_sitter_cli=(tree-sitter)
else
  tree_sitter_cli=(npx --yes tree-sitter-cli@0.25.10)
fi

(
  cd "$GRAMMAR"
  "${tree_sitter_cli[@]}" generate
  "${tree_sitter_cli[@]}" test
  for source in "$ROOT"/schemas/*.bsol; do
    output="$("${tree_sitter_cli[@]}" parse "$source" 2>&1)" || {
      printf '%s\n' "$output" >&2
      exit 1
    }
    if [[ "$output" == *ERROR* || "$output" == *MISSING* ]]; then
      printf '%s\n' "$output" >&2
      exit 1
    fi
  done
)
