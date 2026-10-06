#!/bin/sh
set -eu

expected='96f2fa883334ff15400f51f43bee917e690030e8f3e31b1d8dcdcca2782ab77b'
source_commit='b26db96bca4586b42bd6e2d7741e29bb12f411b1'
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
