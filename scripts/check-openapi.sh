#!/bin/sh
set -eu

expected='d1f223342ad1ca326ba716af6e508c78594e1b108958cce2ec4a1efd31a9773a'
source_commit='1daaf57b8c8bb7481b7c8633a68705428de1f90a'
contract="${1:-openapi.yaml}"

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$contract" | awk '{print $1}')"
else
  actual="$(shasum -a 256 "$contract" | awk '{print $1}')"
fi

if [ "$actual" != "$expected" ]; then
  printf 'OpenAPI drift: expected %s, got %s (%s)\n' "$expected" "$actual" "$contract" >&2
  exit 1
fi

printf 'OpenAPI contract verified at source commit %s (%s).\n' "$source_commit" "$actual"

