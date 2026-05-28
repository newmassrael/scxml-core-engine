#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Verify-before-ship gate for the borrowed zero-copy codec round:
# proves that SCE-generated scalar codecs compile under `no_std`
# WITHOUT the `alloc` feature. The generated codec structs hold
# borrowed zero-copy views (`&'a [u8]` / `&'a str`) and gate every
# `Vec` / `String` / `VecSink` / `encode_to_vec` site behind
# `#[cfg(feature = "alloc")]`, so the decode + sink-based encode path
# is reachable on a heap-free MCU target. This is the SCE-side mirror
# of the consumer build that motivated the round (watching-zenoh's
# codec crate failing `--no-default-features`).
#
# Scope: scalar codecs (Tail / LengthRef byte views) AND list-bearing
# codecs (RFC §5.B B2 repeat / B3 tlv-chain). The latter are now heap-
# free via `heapless::Vec<Body<'a>, MAX>` bounded inline storage (the
# Rust mirror of the C11 `T elems[MAX]; len` shape), so they compile
# under `no_std` without `alloc` too — list elements decode into fixed
# capacity, with `CodecError::TooManyElements` on overflow instead of a
# heap grow. Single-codec embed needs no list storage and was already
# no-alloc after the scalar round.
#
# Usage (from repo root):  scripts/check_noalloc_codec.sh

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

EXPECTED="tests/forge/expected"
RUNTIME="$REPO_ROOT/sce-forge-runtime/rust"

# Build dir under target/ (gitignored) so the workspace stays clean.
WORK="$REPO_ROOT/target/noalloc_codec_check"
rm -rf "$WORK"
mkdir -p "$WORK/src"

# List-free borrowed scalar codecs. Each is its own module file loaded
# via `#[path] mod`, which keeps the golden's leading `#![doc]` inner
# attribute valid (textual `include!` would reject it).
CODECS=(
    codec_tail
    codec_length_ref_uint16_le
    codec_length_ref_uint16_be
    codec_length_ref_uint32_le
    # List-bearing codecs (RFC §5.B B2 repeat / B3 tlv-chain) — no-alloc
    # via heapless::Vec bounded inline storage. Each pulls its element
    # codec as a sibling module (the golden's `use super::<elem>`).
    codec_repeat_elem
    codec_repeat_basic
    codec_until_eof_basic
    codec_tlv_entry
    codec_tlv_chain_basic
)

cat > "$WORK/Cargo.toml" <<EOF
[package]
name = "sce_noalloc_codec_check"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
sce-forge-runtime = { path = "$RUNTIME", default-features = false }

# Declared (but left OFF for this gate) so the codec goldens'
# \`#[cfg(feature = "alloc")]\` blocks are recognized cfg rather than
# "unexpected" — keeps the no-alloc build warning-clean.
[features]
alloc = ["sce-forge-runtime/alloc"]

[workspace]
EOF

{
    echo '#![no_std]'
    echo '#![allow(dead_code)]'
    for c in "${CODECS[@]}"; do
        echo "#[path = \"${c}.rs\"]"
        echo "pub mod ${c};"
    done
} > "$WORK/src/lib.rs"

for c in "${CODECS[@]}"; do
    cp "$EXPECTED/${c}.rs" "$WORK/src/${c}.rs"
done

echo "==> Compiling ${#CODECS[@]} borrowed scalar codecs with --no-default-features (no_std, no alloc)"
( cd "$WORK" && cargo build --no-default-features )

echo "OK: scalar + list-bearing (repeat / tlv-chain) codecs compile no_std without alloc."
rm -rf "$WORK"
