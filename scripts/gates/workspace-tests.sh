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
# `--features cli` is load-bearing, not decoration. sce-build declares test
# targets with `required-features = ["cli"]`; cargo excludes an
# unmet-features target SILENTLY — never built, never reported as skipped —
# so without the flag this gate ran a strictly smaller suite than its name
# claims. How many such targets there are is derived by
# `cli_feature_gating` rather than restated here, and that gate is also
# what fails if a command reaching one of them drops the flag.
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

sce_gate_http_fixture_server

# `SCE_GATE_NO_FAIL_FAST` is the one thing the CI lane wanted that this gate
# did not offer, and it kept the lane restating the command instead of calling
# it. It is a reporting choice, not a different verification: a remote run
# nobody can iterate on wants every failure in one go, a developer wants the
# first one now. Making it a switch is what let the lane delegate.
FAIL_FAST=()
case "${SCE_GATE_NO_FAIL_FAST:-}" in
    "" | 0 | false) ;;
    *) FAIL_FAST=(--no-fail-fast); sce_gate_step "reporting every failure (--no-fail-fast)" ;;
esac

cargo test --workspace --features cli ${FAIL_FAST+"${FAIL_FAST[@]}"} \
    || sce_gate_fail "cargo test --workspace --features cli"
