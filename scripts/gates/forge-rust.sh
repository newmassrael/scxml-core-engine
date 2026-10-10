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
source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"

# The queue arm's stress runs write the histories they recorded here, in the
# JSON form every backend writes (SCE Protocol-Synthesis RFC §synth-5-P,
# verification layer 2). They are judged below by the command the other
# backends' histories are judged by, so no backend is judged by a checker of
# its own. Under `target/`, emptied first: a file left from an earlier run
# would be judged in place of one this run did not write.
QUEUE_HISTORIES="$SCE_REPO_ROOT/target/queue-histories/rust"
rm -rf "$QUEUE_HISTORIES"
mkdir -p "$QUEUE_HISTORIES"

SCE_QUEUE_HISTORY_DIR="$QUEUE_HISTORIES" \
    cargo test --release -p sce-forge-conformance --features alloc \
    || sce_gate_fail "Rust forge conformance"

# Ten shapes are recorded (four Lamport capacities and six SCQ shapes). A glob
# that matched nothing would hand the command a literal pattern and fail, but a
# run that recorded three of ten would pass, so the count is held as well.
shopt -s nullglob
queue_histories=("$QUEUE_HISTORIES"/*.json)
shopt -u nullglob
(( ${#queue_histories[@]} >= 10 )) \
    || sce_gate_fail "Rust queue histories: ${#queue_histories[@]} written, expected at least 10"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-queue-history "${queue_histories[@]}" \
    || sce_gate_fail "Rust queue histories are not linearizable"

# The queue runtime's loom models (SCE Protocol-Synthesis RFC §synth-5-P,
# verification layer 3). They build only under `--cfg loom` and the target is
# `test = false`, so no other lane reaches them; this is the one that does.
# The cfg goes in as a cargo `--config`, not RUSTFLAGS, so this line and the
# casefiles `an_spsc_queue_orders_every_slot_hand_over.cases` and
# `an_scq_queue_orders_every_index_hand_over.cases` spell one command each,
# and `target/loom` keeps the cfg from invalidating the release build above.
cargo test --release -p sce-forge-runtime --target-dir target/loom \
    --config 'build.rustflags=["--cfg","loom"]' \
    --test loom_queue_spsc --test loom_queue_scq --test loom_queue_lscq \
    --test loom_queue_lscq_race \
    || sce_gate_fail "Rust forge queue loom models"

# The queue runtime under Miri and ThreadSanitizer (RFC §synth-5-P,
# verification layer 5). Both need a nightly, and a floating `nightly` would
# turn an unrelated compiler regression into a red here at a time nobody chose,
# so the date is pinned. Moving it is one edit; it is the date on which both
# tools were last run against `queue_threads` and the unit tests and found
# able to see a weakened release store (the SPSC producer's, measured
# 2026-10-09: Miri reports the race as undefined behaviour and ThreadSanitizer
# as a data race, the process exiting non-zero while the test prints `ok`).
# What neither sees is a lost or duplicated element, which is layers 2 and 3.
# The date itself is in `scripts/lib/queue_sanitizer.sh`, which a mutation
# casefile reads too, so that the gate and the casefile cannot disagree.
source "$SCE_REPO_ROOT/scripts/lib/queue_sanitizer.sh"
sce_gate_step "queue runtime under Miri and ThreadSanitizer"
rustup toolchain install "$QUEUE_SANITIZER_NIGHTLY" --profile minimal \
    --component miri --component rust-src >/dev/null 2>&1 \
    || sce_gate_fail "install $QUEUE_SANITIZER_NIGHTLY with miri and rust-src"
cargo "+$QUEUE_SANITIZER_NIGHTLY" miri test -p sce-forge-runtime --lib queue:: \
    || sce_gate_fail "Rust forge queue unit tests under Miri"
# Several seeds, because Miri's weak-memory emulation picks which store a load
# sees from the seed and one seed explores one choice.
MIRIFLAGS="-Zmiri-many-seeds=0..8" \
    cargo "+$QUEUE_SANITIZER_NIGHTLY" miri test -p sce-forge-runtime \
    --test queue_threads \
    || sce_gate_fail "Rust forge queue threads under Miri"
# `-Zbuild-std` because ThreadSanitizer needs the standard library
# instrumented too, or every use of a std lock or channel reads as a race. Its
# own target directory, so the flag does not invalidate the ordinary build.
cargo "+$QUEUE_SANITIZER_NIGHTLY" test -Zbuild-std \
    --target x86_64-unknown-linux-gnu --target-dir target/tsan \
    --config 'build.rustflags=["-Zsanitizer=thread"]' \
    -p sce-forge-runtime --test queue_threads \
    || sce_gate_fail "Rust forge queue threads under ThreadSanitizer"
