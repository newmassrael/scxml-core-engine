#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: sce-rust-runtime-no-std.yml
#
# no_std MCU-target mirror (sce-rust-runtime-no-std.yml).
#
# `clippy` only lints the host target with default features, so it never
# sees the `heapless`-sized types the `no_std` cfg selects — a regression
# that compiles on host but breaks the bare-metal build (a reintroduced
# `std::*` import, an alloc-coupled emission, or a no_std-only clippy lint
# like result_large_err) slips every host-target gate and surfaces only on
# the cloud no_std workflow. This mirrors that workflow: build + clippy the
# runtime for thumbv7em, then generate and compile the allocator-free probe
# machines. `rustup target add` is idempotent — a no-op when the target is
# present, an auto-install otherwise — so the gate always runs rather than
# silently skipping on a fresh checkout.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

rustup target add thumbv7em-none-eabihf >/dev/null 2>&1 || true

sce_gate_step "no_std runtime build + clippy"
cargo build -p sce-rust-runtime --no-default-features --features=no_std \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "no_std runtime build"
cargo clippy -p sce-rust-runtime --no-default-features --features=no_std \
    --target=thumbv7em-none-eabihf -- -D warnings \
    || sce_gate_fail "no_std runtime clippy"

sce_gate_step "allocator-free probe machine"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-build/src \
    sce-build/tests/fixtures/no_std/parallel_history_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "no_std probe generation"
cargo build --manifest-path backends/rust/probes/nostd-build/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "no_std probe compile"

# Single-emit portability: ONE --no-std emission compiles against BOTH the
# std runtime (host, AP profile) and the no_std runtime (thumb, MCU
# profile). Regression gate for the bytes-field `E0433: cannot find
# heapless in sce_rust_runtime` against the std runtime.
# The native-action probe: a machine whose `<sce:action>` lowers to a
# host call has to compile under no_std on the MCU target too. CI ran
# this and the gate did not, so the push-time check was the weaker of
# the two — the asymmetry `hook_ci_parity` exists to catch, and the one
# it could not see: both sides spell the step `cargo build
# --manifest-path ...`, and which source was generated into that crate
# is not a token the extractor compares.
sce_gate_step "native-action probe (no_std thumb)"
NATIVE_ACTION_OUT="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$NATIVE_ACTION_OUT'"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir "$NATIVE_ACTION_OUT" \
    sce-build/tests/fixtures/event_schema/statechart_native_action.scxml >/dev/null 2>&1 \
    || sce_gate_fail "native-action probe generation"
cp "$NATIVE_ACTION_OUT/statechart_native_action_sm.rs" \
    backends/rust/probes/nostd-build/src/parallel_history_probe_sm.rs \
    || sce_gate_fail "native-action probe staging"
cargo build --manifest-path backends/rust/probes/nostd-build/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "native-action probe compile"

sce_gate_step "single-emit portability probe (std host + no_std thumb)"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/portable-emit/src \
    sce-build/tests/fixtures/no_std/portable_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "portability probe generation"
cargo build --manifest-path backends/rust/probes/portable-emit/Cargo.toml \
    || sce_gate_fail "portability probe — std runtime, host"
cargo build --manifest-path backends/rust/probes/portable-emit/Cargo.toml \
    --target=thumbv7em-none-eabihf --features mcu \
    || sce_gate_fail "portability probe — no_std runtime, thumb"

# Per-machine event-queue sizing: a `<scxml sce:capacity="2">` machine must
# size its no_std queues to depth 2, not the crate-global depth-64 default.
# The const size assertion catches a silent revert of the per-machine
# `StatePolicy::EventQueue` to the bare (depth-64) form — which a plain
# compile gate misses, since the bare form still compiles. This is the
# downstream MCU SRAM blocker class.
sce_gate_step "per-machine event-queue sizing"
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-queue-size/machine/src \
    sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "queue-size probe generation"
cargo build --manifest-path backends/rust/probes/nostd-queue-size/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "queue-size gate — const size assertion"

# What the engine COSTS an MCU image, which every step above this line is
# blind to. They ask whether generated code compiles without an allocator; a
# generic function that nothing instantiates compiles fine and weighs nothing,
# so `Engine<P>` had never been monomorphised for an MCU target in this
# repository at all — `nm` on the probe rlib returns zero engine symbols.
#
# The consequence was measured from outside on 2026-08-21: a bare-metal
# consumer reported `Engine<..>::run_main_event_loop` growing 60-198 bytes per
# instantiation across two SCE pins, multiplied by the five machines its image
# holds, and no gate here could confirm or deny a byte of it. Their firmware
# was the only witness in existence.
#
# `nostd-footprint` is a staticlib that actually drives an engine, so the code
# exists and can be weighed. The budget beside it is measured, two-sided, and
# carries a symbol floor — the floor because the probe's first shape emitted
# LLVM bitcode and `nm` saw nothing, which a one-sided gate would have called
# green forever.
sce_gate_step "MCU footprint budget (Engine<P> .text on thumb)"
# Regenerated here rather than trusted: the native-action step above stages a
# DIFFERENT machine into this same path, so whichever ran last decides what the
# probe crate is. The instrument names `ParallelHistoryProbePolicy`.
"$CODEGEN" --workspace-root "$SCE_REPO_ROOT" generate \
    -l rust --no-std --output-dir backends/rust/probes/nostd-build/src \
    sce-build/tests/fixtures/no_std/parallel_history_probe.scxml >/dev/null 2>&1 \
    || sce_gate_fail "footprint probe generation"
cargo build --release --manifest-path backends/rust/probes/nostd-footprint/Cargo.toml \
    --target=thumbv7em-none-eabihf \
    || sce_gate_fail "footprint probe compile"

FOOTPRINT_LIB="backends/rust/probes/nostd-footprint/target/thumbv7em-none-eabihf/release/libsce_nostd_footprint_probe.a"
FOOTPRINT_BUDGET="backends/rust/probes/nostd-footprint/footprint.budget"
# shellcheck source=/dev/null
source "$FOOTPRINT_BUDGET"

# One configuration: build it, weigh every Engine<..> symbol, hold the sum to
# its budget in BOTH directions. A drop is a failure too — the probe's first
# shape emitted LLVM bitcode and `nm` reported nothing, and a gate that only
# watched for growth would have called that green forever.
sce_footprint_weigh() {
    local label="$1" budget="$2"; shift 2
    cargo build --release --manifest-path backends/rust/probes/nostd-footprint/Cargo.toml \
        --target=thumbv7em-none-eabihf "$@" \
        || sce_gate_fail "footprint probe compile ($label)"
    [[ -f "$FOOTPRINT_LIB" ]] \
        || sce_gate_fail "footprint probe produced no staticlib at $FOOTPRINT_LIB ($label)"

    local syms bytes lo hi
    read -r syms bytes < <(
        nm --print-size --size-sort -C "$FOOTPRINT_LIB" 2>/dev/null |
            awk '/sce_rust_runtime::engine::Engine</ { n++; s += strtonum("0x" $2) }
                 END { printf "%d %d\n", n, s }'
    )

    if (( syms < MIN_SYMBOLS )); then
        printf '  [nostd-mcu] %s: found %d Engine<..> symbol(s), floor is %d\n' \
            "$label" "$syms" "$MIN_SYMBOLS" >&2
        printf '  [nostd-mcu] the instrument is not measuring the engine — a renamed symbol,\n' >&2
        printf '  [nostd-mcu] a bitcode staticlib, or a driver that stopped driving.\n' >&2
        sce_gate_fail "MCU footprint instrument is blind ($label)"
    fi

    lo=$(( budget * (100 - TOLERANCE_PCT) / 100 ))
    hi=$(( budget * (100 + TOLERANCE_PCT) / 100 ))
    printf '  [nostd-mcu] %-16s %5d byte(s) over %2d symbol(s) (budget %d, band %d..%d)\n' \
        "$label:" "$bytes" "$syms" "$budget" "$lo" "$hi"

    if (( bytes > hi || bytes < lo )); then
        printf '  [nostd-mcu] per-symbol breakdown:\n' >&2
        nm --print-size --size-sort -C "$FOOTPRINT_LIB" 2>/dev/null |
            grep 'sce_rust_runtime::engine::Engine<' >&2 || true
        printf '  [nostd-mcu] re-pin the budget in %s if the change is meant, and say\n' \
            "$FOOTPRINT_BUDGET" >&2
        printf '  [nostd-mcu] in the commit message what the bytes bought.\n' >&2
        sce_gate_fail "MCU footprint outside its budget ($label)"
    fi
}

# As shipped, then with the macrostep report compiled out. The second build is
# not only a second number: an opt-out feature nothing builds is one that stops
# compiling and nobody finds out until a consumer sets it.
sce_footprint_weigh "diagnostics on" "$TOTAL_BYTES"
sce_footprint_weigh "diagnostics off" "$TOTAL_BYTES_NO_DIAGNOSTICS" \
    --features no-macrostep-diagnostics
