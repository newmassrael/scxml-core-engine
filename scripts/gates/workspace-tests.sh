#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: rust-workspace-tests.yml
#
# Rust workspace suite (mirror of rust-workspace-tests.yml).
#
# Default test profile, matching CI. This ran `--release` until 2026-08-04,
# which was weaker and slower at once. Weaker because `[profile.test]` keeps
# debug-assertions on and release turns them off: an integer overflow panics
# for CI and wraps here, so the one gate meant to catch it before the runner
# could not. Slower because development builds are unoptimised, so a release
# sweep shared no artifacts with the tree and rebuilt the workspace every
# push. `hook_ci_parity` (in `tree-hygiene`) fails if either side drifts back.
#
# `--features cli` is load-bearing, not decoration. sce-build declares 15
# test targets with `required-features = ["cli"]`; cargo excludes an
# unmet-features target SILENTLY — never built, never reported as skipped —
# so without the flag this gate ran a strictly smaller suite than its name
# claims.
#
# The W3C HTTP fixture server is started here rather than by the runner.
# Rust integration tests for W3C SCXML C.2 BasicHTTPEventProcessor
# (test_201, test_509, test_513, test_518-520, test_532, test_534,
# test_567) issue real HTTP POSTs to localhost:8080/test and panic on
# connection-refused without it; w3c-tests.yml starts the same server
# before its own `cargo test` step. Owning it here is what makes this gate
# runnable on its own — under the previous arrangement the server was set
# up by the hook, so running the suite by hand meant remembering to start
# it, and remembering to stop it afterwards (a live 8080 makes the C++
# ctest suite report 13 tests Not Run).

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v node >/dev/null 2>&1 \
    || sce_gate_fail "node.js required for the W3C HTTP fixture server (apt install nodejs)"

HTTP_LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$HTTP_LOG'"
node tests/w3c/standalone_http_server.js 8080 /test >"$HTTP_LOG" 2>&1 &
HTTP_PID=$!
sce_gate_on_exit "kill $HTTP_PID 2>/dev/null"
# Match the CI workflow's settle window before issuing requests.
sleep 1
if ! kill -0 "$HTTP_PID" 2>/dev/null; then
    cat "$HTTP_LOG" >&2
    sce_gate_fail "W3C HTTP fixture server failed to start (port 8080 already in use?)"
fi
sce_gate_step "W3C HTTP fixture server up on localhost:8080"

cargo test --workspace --features cli \
    || sce_gate_fail "cargo test --workspace --features cli"
