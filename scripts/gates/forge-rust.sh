#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Rust arm of the forge conformance suite (forge-conformance.yml).
#
# `--release` is not incidental: the numerical conformance vectors are
# compared against optimised floating-point output, which is what ships.
# `workspace-tests` runs the workspace in debug and cannot stand in for it.
# The separate profile costs a one-time release build; afterwards it is
# incremental — near-zero on a push that does not touch the crate, which is
# most of them.
#
# Every target of `sce-forge-conformance` runs, not only the numerical one.
# The codec round-trip tests beside it moved out of `sce-forge-runtime` with
# the build script that generates their fixtures, and until then this gate
# named `--test numerical_conformance` alone while `workspace-tests` builds
# without `alloc` — so no lane ran them at all.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo test --release -p sce-forge-conformance --features alloc \
    || sce_gate_fail "Rust forge conformance"

# The queue runtime's loom models (SCE Protocol-Synthesis RFC §synth-5-P,
# verification layer 3). They build only under `--cfg loom` and the target is
# `test = false`, so no other lane reaches them; this is the one that does.
# The cfg goes in as a cargo `--config`, not RUSTFLAGS, so this line and the
# casefiles `an_spsc_queue_orders_every_slot_hand_over.cases` and
# `an_scq_queue_orders_every_index_hand_over.cases` spell one command each,
# and `target/loom` keeps the cfg from invalidating the release build above.
cargo test --release -p sce-forge-runtime --target-dir target/loom \
    --config 'build.rustflags=["--cfg","loom"]' \
    --test loom_queue_spsc --test loom_queue_scq \
    || sce_gate_fail "Rust forge queue loom models"
