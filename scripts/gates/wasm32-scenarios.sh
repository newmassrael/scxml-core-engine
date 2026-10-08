#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: wasm32-scenarios.yml
#
# The Rust engine, run as the module a browser loads.
#
# Every other lane judges the engine on a host that has an operating system. A
# `wasm32-unknown-unknown` module has none: the target gives it no clock, no
# threads and no files, and the failures that follow are not compile errors. The
# engine once read the wall clock at its first `initialize()`, which compiles on
# that target and traps when the module runs -- so every native build and every
# test passed while no machine could start in a browser.
#
# This gate builds the scenario replay crate for that target and runs it under
# Node and in headless Chrome, one fresh instance per scenario. Chrome is the web
# target's own host and Node the faster one; the same module has to pass in both.
# The scenarios are the engine-neutral
# files every backend replays (`sce-build/tests/fixtures/static_datamodel/
# scenarios/*.json`), so the module is held to the answers the native engine, the
# C++ one and the other four are held to, and not to answers of its own.
#
# The module is built WITHOUT telling the engine a clock. The replay installs a
# manual clock only for the scenarios of a delayed send, so the rest run on
# whatever the engine starts with on this target. That is the point: the check
# that the default is host-owned time is the 41 scenarios that never set one.
#
# `wasm-bindgen` is the command-line tool and not a library the build links, and
# the glue it writes and the module it reads are only compatible at one version.
# The version is the one `Cargo.lock` holds for the library, and a tool at any
# other is refused rather than tried: the error a mismatched pair gives is a
# schema complaint from inside the module, nothing that points at the tool.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

TARGET="wasm32-unknown-unknown"
CRATE="sce-rust-scenarios"
NODE_RUNNER="backends/rust/scenarios/wasm/run.mjs"
CHROME_RUNNER="backends/rust/scenarios/wasm/run_chrome.mjs"

# The target's standard library is a rustup component, not part of the
# toolchain: a machine that has a compiler and not this component fails inside
# cargo with "can't find crate for `core`", which reads as a fault in the tree.
[[ -d "$(rustc --print target-libdir --target "$TARGET" 2>/dev/null)" ]] \
    || sce_gate_cannot_run "the $TARGET standard library is not installed (rustup target add $TARGET)"

command -v node >/dev/null 2>&1 \
    || sce_gate_cannot_run "node is not on PATH; the module is run under Node"

command -v wasm-bindgen >/dev/null 2>&1 \
    || sce_gate_cannot_run "wasm-bindgen is not on PATH (cargo install wasm-bindgen-cli --version <the one Cargo.lock holds> --locked)"

locked="$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')" || locked=""
[[ -n "$locked" && "$locked" != *$'\n'* ]] \
    || sce_gate_fail "Cargo.lock holds no single wasm-bindgen version (read: '${locked}'), so the tool to match is not defined"
[[ "$(wasm-bindgen --version)" == "wasm-bindgen ${locked}" ]] \
    || sce_gate_cannot_run "wasm-bindgen is '$(wasm-bindgen --version)' and Cargo.lock holds ${locked}; install the tool at that version"

# The lint half: the runtime and the replay crate, compiled for the target. A
# cfg-gated branch exists for this target and no other, so the host's clippy
# never reads it, and a call into the operating system in one is exactly what
# the runtime's `clippy.toml` bans. `-D warnings` as the workspace lint does.
sce_gate_step "clippy for $TARGET"
cargo clippy -p "$CRATE" --target "$TARGET" -- -D warnings \
    || sce_gate_fail "cargo clippy -p $CRATE --target $TARGET"

# The dev profile, not release. Release is `lto = "thin"` with one codegen unit
# (Cargo.toml), which sends the 44 generated machines through a single unit:
# measured 2026-10-08 it took 10m15s to build this crate, for a run that lasts a
# second and gains nothing from optimisation. The dev profile also keeps debug
# assertions and overflow checks on, as the native test profile does, so an
# arithmetic overflow the optimiser would have wrapped is a trap here.
sce_gate_step "building $CRATE for $TARGET"
cargo build -p "$CRATE" --target "$TARGET" \
    || sce_gate_fail "cargo build -p $CRATE --target $TARGET"

module="${CARGO_TARGET_DIR:-$SCE_REPO_ROOT/target}/$TARGET/debug/${CRATE//-/_}.wasm"
node_glue="$(mktemp -d)"
web_glue="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$node_glue' '$web_glue'"

# One module, two glues: the tool writes a different loader for each host, and a
# module it has processed for one is not the file the other loads.
sce_gate_step "writing the glue for Node and for the web (wasm-bindgen ${locked})"
wasm-bindgen --target nodejs --out-dir "$node_glue" "$module" \
    || sce_gate_fail "wasm-bindgen (nodejs) could not read the module cargo built"
wasm-bindgen --target web --out-dir "$web_glue" "$module" \
    || sce_gate_fail "wasm-bindgen (web) could not read the module cargo built"

sce_gate_step "replaying every scenario in the module under Node"
node "$NODE_RUNNER" "$node_glue" \
    || sce_gate_fail "a scenario failed in the $TARGET module under Node: the engine does not do on that target what the scenario holds it to (each failing scenario and its panic are printed above)"

# The web target is Chrome, so Chrome is the host whose verdict matters; Node is
# the faster one. A machine with no Chrome cannot give that verdict, which is the
# gate's input missing (exit 3) and not a fault of the module.
sce_gate_step "replaying every scenario in the module in Chrome"
chrome_status=0
node "$CHROME_RUNNER" "$web_glue" || chrome_status=$?
case "$chrome_status" in
    0) ;;
    3) sce_gate_cannot_run "no Chrome on PATH (install google-chrome, or name one in CHROME_BIN); the web target is Chrome" ;;
    *) sce_gate_fail "the $TARGET module failed in Chrome, or the page could not report (each failing scenario and its panic, or the reason, are printed above)" ;;
esac
