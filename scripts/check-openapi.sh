#!/bin/sh
set -eu

expected='a58782a840d21315907466baa86734f6bbec8cc5007a6e8ac3fa79c8b94e53d4'
source_commit='f0f33d678b5cd2a9db5ca69b881c373afb3833f9'
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
