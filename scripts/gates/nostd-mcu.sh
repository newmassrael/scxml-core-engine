#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
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
