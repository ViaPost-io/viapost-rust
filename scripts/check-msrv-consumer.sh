#!/bin/sh
set -eu

repository_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
consumer_dir="$(mktemp -d "${TMPDIR:-/tmp}/viapost-rust-msrv.XXXXXX")"
trap 'rm -rf "$consumer_dir"' EXIT INT TERM
cargo_home="$consumer_dir/cargo-home"

cat > "$consumer_dir/Cargo.toml" <<EOF
[package]
name = "viapost-msrv-consumer"
version = "0.0.0"
edition = "2021"
rust-version = "1.85"

[dependencies]
viapost = { path = "$repository_root" }
EOF

mkdir -p "$consumer_dir/src"
mkdir -p "$cargo_home"
cat > "$consumer_dir/src/main.rs" <<'EOF'
use viapost::ViaPost;

fn main() {
    let client = ViaPost::new("vp_test_msrv_consumer").expect("client should be created");
    let _ = client.messages();
}
EOF

(cd "$consumer_dir" && CARGO_HOME="$cargo_home" cargo check)
