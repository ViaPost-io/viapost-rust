#!/bin/sh
set -eu

expected='7c931b5a4a2a602d3c42341f2a70af9c49378600894b31adebfd333469b9e183'
source_commit='2c2eee4c5250b6205338965405a46eb02db4ce2f'
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
