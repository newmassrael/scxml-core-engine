#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: wasm32-scenarios.yml
#
# The Rust lane on wasm32: the engine run as the module a browser loads, and
# the browser node (docs/adr/0117) held to its kit.
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
# The browser node is the HTTPS binding's client core in a module of its own
# (`sce-rust-browser-node`), stepped by the shell the page's script is made of
# (backends/rust/browser-node/js). Its kit holds that shell to the binding's own
# server (the example `https_echo` of `sce-rust-mesh`, built for the host),
# under Node and in headless Chrome and Firefox, the two engines of the kit
# (docs/adr/0117, open question 3): Node on its own fetch, WebCrypto and timers,
# each browser on its IndexedDB and Web Locks too.
#
# The page's surface (docs/adr/0117, D7) is what a page is written against: a
# module and its TypeScript declaration that `sce-codegen surface` writes from a
# document. The pinned TypeScript (scripts/install_typescript.sh) compiles a page
# that uses it right and must refuse, line by line, one whose every use is wrong;
# Node runs the module's checks of what a page in JavaScript hands it
# (backends/rust/browser-node/surface).
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
NODE_CRATE="sce-rust-browser-node"
KIT_NODE_RUNNER="backends/rust/browser-node/kit/run_node.mjs"
KIT_BROWSER_RUNNER="backends/rust/browser-node/kit/run_browser.mjs"
SURFACE_DOCUMENT="sce-build/tests/fixtures/static_datamodel/static_optional.scxml"
SURFACE_RUNNER="backends/rust/browser-node/surface/check.mjs"

# The target's standard library is a part of a toolchain a compiler can be
# without: a machine that has a compiler and not this target fails inside cargo
# with "can't find crate for `core`", which reads as a fault in the tree. The
# Rust release is declared with it (`[platform.linux.tools.rust]` in bx.toml).
sce_gate_require_rust_target "$TARGET"

# TWO PARTS, when `SCE_WASM32_SCENARIOS_SHARD` names one (ADR 0120 D3): 1/2 is
# the lint half below, 2/2 the module built, given its glue and replayed. They
# share nothing but the target's standard library, and each compiles the
# generator crate its own way (clippy's artefacts are not the build's), which
# is why the lane runs them as two jobs: whole, the job took 11 minutes (job
# 139040902859, 2026-11-25), 253 s of clippy and 250 s of the build before 69 s
# of replay. Unset, both run, which is what a developer and the build machines
# run.
LINT=1
REPLAY=1
case "${SCE_WASM32_SCENARIOS_SHARD:-}" in
    "") ;;
    1/2) REPLAY=0 ;;
    2/2) LINT=0 ;;
    *) sce_gate_fail "SCE_WASM32_SCENARIOS_SHARD is '${SCE_WASM32_SCENARIOS_SHARD}': the gate has two parts, 1/2 (clippy for $TARGET) and 2/2 (the module built and replayed)" ;;
esac

if (( LINT )); then
    # The lint half: the runtime and the replay crate, compiled for the target. A
    # cfg-gated branch exists for this target and no other, so the host's clippy
    # never reads it, and a call into the operating system in one is exactly what
    # the runtime's `clippy.toml` bans. `-D warnings` as the workspace lint does.
    sce_gate_step "clippy for $TARGET"
    sce_gate_tested -- cargo clippy -p "$CRATE" --target "$TARGET" -- -D warnings \
        || sce_gate_fail "cargo clippy -p $CRATE --target $TARGET"
    # The browser node's module, and the Mesh host core under the feature
    # `browser` with it: the only build of that core for this target.
    sce_gate_tested -- cargo clippy -p "$NODE_CRATE" --target "$TARGET" -- -D warnings \
        || sce_gate_fail "cargo clippy -p $NODE_CRATE --target $TARGET"
fi
if (( ! REPLAY )); then
    exit 0
fi

command -v node >/dev/null 2>&1 \
    || sce_gate_cannot_run "node is not on PATH; the module is run under Node"

locked="$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')" || locked=""
[[ -n "$locked" && "$locked" != *$'\n'* ]] \
    || sce_gate_fail "Cargo.lock holds no single wasm-bindgen version (read: '${locked}'), so the tool to match is not defined"

# The tool at that version: the one on PATH when it is that version, else the
# one scripts/install_wasm_bindgen.sh installed, at WASM_BINDGEN_ROOT when it is
# set (the gate's image names the copy it holds there) or at the path derived
# from the version, where bx installs it on a build machine
# (`[platform.linux.tools.wasm-bindgen]`). The version check below refuses any
# other either way.
bindgen_root="${WASM_BINDGEN_ROOT:-$HOME/.local/share/wasm-bindgen/${locked}}"
if [[ "$(wasm-bindgen --version 2>/dev/null)" != "wasm-bindgen ${locked}" ]] \
        && [[ -x "$bindgen_root/bin/wasm-bindgen" ]]; then
    PATH="$bindgen_root/bin:$PATH"
    export PATH
fi
command -v wasm-bindgen >/dev/null 2>&1 \
    || sce_gate_cannot_run "wasm-bindgen is not on PATH (scripts/install_wasm_bindgen.sh installs the version Cargo.lock holds)"
[[ "$(wasm-bindgen --version)" == "wasm-bindgen ${locked}" ]] \
    || sce_gate_cannot_run "wasm-bindgen is '$(wasm-bindgen --version)' and Cargo.lock holds ${locked}; install the tool at that version (scripts/install_wasm_bindgen.sh)"

# The browser: the pinned Chrome for Testing (scripts/install_chrome.sh), which
# bx installs on a build machine and the gate's image holds, each where that
# script installs it: CHROME_ROOT when it is set, as the image sets it, else the
# path derived from the release. A CHROME_BIN the caller set is taken as given.
# Outside a landing a machine without the pinned build may still replay in the
# Chrome on its PATH, as a developer's does; a landing's verdict is the pinned
# browser's or none, since a pass is kept from it under a key that names no
# other Chrome.
chrome_rev="$(sed -n 's/^CHROME_REV="\([0-9.][0-9.]*\)"$/\1/p' scripts/install_chrome.sh)"
chrome_root="${CHROME_ROOT:-$HOME/.local/share/chrome-for-testing/${chrome_rev}}"
pinned_chrome="$chrome_root/chrome-linux64/chrome"
if [[ -z "${CHROME_BIN:-}" && -n "$chrome_rev" && -x "$pinned_chrome" ]]; then
    export CHROME_BIN="$pinned_chrome"
fi
if sce_gate_landing && [[ "${CHROME_BIN:-}" != "$pinned_chrome" ]]; then
    sce_gate_cannot_run "the pinned Chrome for Testing ${chrome_rev:-?} is not at $pinned_chrome (scripts/install_chrome.sh). This run is a landing's, where only the pinned browser judges the module."
fi

# Firefox, the kit's second engine, as Chrome is: the pinned release
# (scripts/install_firefox.sh) at FIREFOX_ROOT, as the image sets it, else at
# the path derived from the release; a FIREFOX_BIN the caller set is taken as
# given, and a landing's verdict is the pinned browser's or none.
firefox_rev="$(sed -n 's/^FIREFOX_REV="\([0-9.][0-9.]*\)"$/\1/p' scripts/install_firefox.sh)"
firefox_root="${FIREFOX_ROOT:-$HOME/.local/share/firefox/${firefox_rev}}"
pinned_firefox="$firefox_root/firefox/firefox"
if [[ -z "${FIREFOX_BIN:-}" && -n "$firefox_rev" && -x "$pinned_firefox" ]]; then
    export FIREFOX_BIN="$pinned_firefox"
fi
if sce_gate_landing && [[ "${FIREFOX_BIN:-}" != "$pinned_firefox" ]]; then
    sce_gate_cannot_run "the pinned Firefox ${firefox_rev:-?} is not at $pinned_firefox (scripts/install_firefox.sh). This run is a landing's, where only the pinned browser judges the browser node."
fi

# The dev profile, not release. Release is `lto = "thin"` with one codegen unit
# (Cargo.toml), which sends the 44 generated machines through a single unit:
# measured 2026-10-08 it took 10m15s to build this crate, for a run that lasts a
# second and gains nothing from optimisation. The dev profile also keeps debug
# assertions and overflow checks on, as the native test profile does, so an
# arithmetic overflow the optimiser would have wrapped is a trap here.
sce_gate_step "building $CRATE for $TARGET"
sce_gate_tested -- cargo build -p "$CRATE" --target "$TARGET" \
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
    3) sce_gate_cannot_run "no Chrome on PATH and none named by CHROME_BIN (scripts/install_chrome.sh installs the pinned one); the web target is Chrome" ;;
    *) sce_gate_fail "the $TARGET module failed in Chrome, or the page could not report (each failing scenario and its panic, or the reason, are printed above)" ;;
esac

# The browser node: its module for the target, its two glues as the scenarios'
# have, and the binding's own server for the host, which its kit runs against.
sce_gate_step "building $NODE_CRATE for $TARGET, and the binding's echo server"
sce_gate_tested -- cargo build -p "$NODE_CRATE" --target "$TARGET" \
    || sce_gate_fail "cargo build -p $NODE_CRATE --target $TARGET"
sce_gate_tested -- cargo build -p sce-rust-mesh --features https --example https_echo \
    || sce_gate_fail "cargo build -p sce-rust-mesh --features https --example https_echo"

node_module="${CARGO_TARGET_DIR:-$SCE_REPO_ROOT/target}/$TARGET/debug/${NODE_CRATE//-/_}.wasm"
echo_server="${CARGO_TARGET_DIR:-$SCE_REPO_ROOT/target}/debug/examples/https_echo"
node_kit_glue="$(mktemp -d)"
web_kit_glue="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$node_kit_glue' '$web_kit_glue'"
wasm-bindgen --target nodejs --out-dir "$node_kit_glue" "$node_module" \
    || sce_gate_fail "wasm-bindgen (nodejs) could not read the browser node's module"
wasm-bindgen --target web --out-dir "$web_kit_glue" "$node_module" \
    || sce_gate_fail "wasm-bindgen (web) could not read the browser node's module"

sce_gate_step "running the browser node's kit under Node"
node "$KIT_NODE_RUNNER" "$node_kit_glue" "$echo_server" \
    || sce_gate_fail "the browser node's kit failed under Node (each failing case and why are printed above)"

# Both engines give a verdict; a machine without one cannot, which is the
# gate's input missing and not a fault of the node.
for engine in chrome firefox; do
    sce_gate_step "running the browser node's kit in $engine"
    kit_status=0
    node "$KIT_BROWSER_RUNNER" "$engine" "$web_kit_glue" "$echo_server" || kit_status=$?
    case "$kit_status" in
        0) ;;
        3) sce_gate_cannot_run "no $engine on PATH and none named by its variable (scripts/install_$engine.sh installs the pinned one); the browser node's kit runs in two engines" ;;
        *) sce_gate_fail "the browser node's kit failed in $engine, or the page could not report (each failing case and why, or the reason, are printed above)" ;;
    esac
done

# The page's surface (docs/adr/0117, D7), written from a document every backend's
# scenarios already read, so it is the surface of a machine that runs. The
# compiler is the pinned TypeScript: the package scripts/install_typescript.sh
# unpacks at TYPESCRIPT_ROOT, as the image sets it, else at the path derived from
# the release, and only that release judges the declaration, since a page that
# must not compile is held to one compiler's reading of it.
sce_gate_step "the page surface of $SURFACE_DOCUMENT, compiled by TypeScript and run under Node"
typescript_rev="$(sed -n 's/^TYPESCRIPT_REV="\([0-9.][0-9.]*\)"$/\1/p' scripts/install_typescript.sh)"
[[ -n "$typescript_rev" ]] \
    || sce_gate_fail "scripts/install_typescript.sh names no TYPESCRIPT_REV, so the compiler that judges the page surface is not defined"
typescript_package="${TYPESCRIPT_ROOT:-$HOME/.local/share/typescript/${typescript_rev}}/package"
typescript_found="$(node -e 'process.stdout.write(require(process.argv[1]).version)' "$typescript_package/package.json" 2>/dev/null)" \
    || typescript_found=""
[[ "$typescript_found" == "$typescript_rev" ]] \
    || sce_gate_cannot_run "TypeScript $typescript_rev is not at $typescript_package (scripts/install_typescript.sh installs it); the page surface's declaration is judged by that compiler and no other"
codegen="$(sce_gate_codegen)" \
    || sce_gate_fail "could not provide sce-codegen, which writes the page surface"
surface_dir="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$surface_dir'"
"$codegen" surface "$SURFACE_DOCUMENT" --out-dir "$surface_dir" \
    || sce_gate_fail "sce-codegen surface refused $SURFACE_DOCUMENT, whose surface a page is written against"
node "$SURFACE_RUNNER" "$surface_dir" "$typescript_package" \
    || sce_gate_fail "the page surface failed: a page that uses it right did not compile, a wrong use compiled, or its module admitted or refused what it must not (each is printed above)"
