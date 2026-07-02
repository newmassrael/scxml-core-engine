#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Clippy gate for SCE-generated Rust codecs. Proves that EVERY committed
# codec golden passes `cargo clippy -- -D warnings` with the `alloc`
# feature ON — the configuration a downstream consumer (e.g.
# watching-zenoh) builds them in. SCE's workspace clippy gate
# (clippy-check.yml) only covers hand-written `src/`; the generated codec
# `.rs` are golden text files, not workspace members, so without this gate
# clippy regressions in the codegen reach the consumer undetected (the
# rustc-green / consumer-clippy-red gap). No blanket `#![allow]` — the
# codegen emits clippy-clean Rust; the only suppression is a targeted
# per-item `#[allow(clippy::eq_op, clippy::assertions_on_constants)]` on the
# deliberate DMA-alignment compile-time drift guard, which clippy cannot
# model (it sees only codegen-baked integer literals).
#
# Complements check_noalloc_codec.sh, which proves the borrowed path
# compiles `--no-default-features`; this gate proves the alloc-ON path
# (owned projection included) is lint-clean.
#
# Usage (from repo root):  scripts/check_clippy_codec.sh

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

EXPECTED="tests/forge/expected"
RUNTIME="$REPO_ROOT/backends/rust/forge-runtime"

# Build dir under target/ (gitignored) so the workspace stays clean.
WORK="$REPO_ROOT/target/clippy_codec_check"
rm -rf "$WORK"
mkdir -p "$WORK/src"

cat > "$WORK/Cargo.toml" <<EOF
[package]
name = "sce_clippy_codec_check"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
sce-forge-runtime = { path = "$RUNTIME", default-features = false }

# alloc ON: matches the downstream consumer build and exercises the
# alloc-gated owned projection (\`{Codec}Owned\` / \`into_owned\`) as well as
# the borrowed views. Declaring the feature also makes the goldens'
# \`#[cfg(feature = "alloc")]\` blocks recognized cfg rather than "unexpected".
[features]
default = ["alloc"]
alloc = ["sce-forge-runtime/alloc"]

[workspace]
EOF

# Every committed codec golden, EXCEPT the *_test.rs round-trip sidecars
# (those are test harnesses, not codecs). Each loads as its own
# `#[path] mod` so the golden's leading `#![doc]` inner attribute stays
# valid and any sibling `use super::<elem>` element-codec reference
# resolves against the shared crate root.
{
    echo '#![allow(dead_code)]'
    for f in "$EXPECTED"/codec_*.rs; do
        b="$(basename "$f" .rs)"
        case "$b" in *_test) continue;; esac
        echo "#[path = \"${b}.rs\"]"
        echo "pub mod ${b};"
    done
} > "$WORK/src/lib.rs"

count=0
for f in "$EXPECTED"/codec_*.rs; do
    b="$(basename "$f" .rs)"
    case "$b" in *_test) continue;; esac
    cp "$f" "$WORK/src/${b}.rs"
    count=$((count + 1))
done

echo "==> Linting ${count} generated codec goldens with cargo clippy -D warnings (alloc on)"
( cd "$WORK" && cargo clippy --quiet -- -D warnings )

echo "OK: all ${count} generated codecs pass cargo clippy -D warnings (alloc on)."
rm -rf "$WORK"
