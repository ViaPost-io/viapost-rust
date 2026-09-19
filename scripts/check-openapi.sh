#!/bin/sh
set -eu

expected='c5d5ae1d85e61b4e14e09351b14146465ce357075d2ed5fe4e034f6ff6693dc1'
source_commit='2b9f310f2f2a737e1eed9ba68c99a8d1b56b6d10'
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
