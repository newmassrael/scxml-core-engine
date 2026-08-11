#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: doc-check.yml
#
# cargo doc broken-intra-doc-link gate (mirror of doc-check.yml).
#
# sce-rust-runtime opts into the workspace `[lints]`
# broken-intra-doc-links=deny policy, but that policy only bites when
# something runs `cargo doc`. Both feature profiles are gated: a doc link
# that crosses a `#[cfg]` boundary (e.g. into a `!no_std`-gated module)
# resolves in the default-features docs yet breaks in the no_std docs, so
# the std-only run cannot see that breakage class. All runtime doc links are
# kept profile-stable (cfg-gated targets are plain code spans).
#
# The registry maps this gate to doc-check.yml. Under the numbered stage
# table it was mapped to sce-rust-runtime-no-std.yml instead, while this
# comment already named doc-check.yml — the two workflows share the
# `backends/rust/runtime/**` trigger, so the mismatch never showed as a
# missed run except for edits to doc-check.yml itself.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

RUSTDOCFLAGS="-D warnings" cargo doc -p sce-rust-runtime --no-deps \
    || sce_gate_fail "broken intra-doc links (std profile)"
RUSTDOCFLAGS="-D warnings" cargo doc -p sce-rust-runtime --no-deps \
    --no-default-features --features no_std \
    || sce_gate_fail "broken intra-doc links (no_std profile)"
