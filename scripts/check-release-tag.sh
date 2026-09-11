#!/bin/sh
set -eu

tag="${1:?usage: scripts/check-release-tag.sh vX.Y.Z}"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
test -n "$version"
test "$tag" = "v$version"
printf 'Release tag %s matches Cargo package version.\n' "$tag"

