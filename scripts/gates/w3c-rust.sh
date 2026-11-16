#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Rust AOT conformance arm: the W3C cases, the modules of the one test
# target `w3c` (`backends/rust/tests/tests/w3c.rs`), plus the hand-written tests
# beside them in `sce-rust-tests`.
#
# It was the one W3C arm with no gate. The `test-rust` job spelled its
# generation, its fixture server and its `cargo test` inline, while the Go,
# Python, Kotlin, C++ and C11 jobs each run `scripts/gate w3c-<backend>`. A
# backend's lanes are read from the gate registry by the rule
# `<family>-<canonical name>`, so a lane with no gate was a row nobody could
# derive. The job runs this script now, and the commands live here once.
#
# The Rust W3C tree is a build output (docs/adr/0097): the crate's build
# script writes it into OUT_DIR from the generator in the working tree, so the
# build below judges the generator as it stands, with no generation step here.
#
# `--release` and `--no-fail-fast`, as the lane ran it. The same cases also run
# in the test profile inside `workspace-tests`; this is the arm that runs them
# optimised, the way the crate's own documentation runs it.
#
# The BasicHTTP fixtures need the echo server, and the harness addresses
# whatever port `SCE_W3C_HTTP_PORT` names, which the fixture helper exports
# once the server reports itself ready. The lane used to start it with a
# backgrounded `node` and a one-second sleep, the timer `lib.sh` records as
# answering before the port was bound.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# The population the run is held to, read from what the build placed rather
# than written here. Each W3C case is one module of the target `w3c` holding
# one `#[test]` named after it (`test_403a::test_403a`), so the cases the
# target lists are the denominator and the passing case lines the numerator.
#
# The floor under the denominator is the one the Go and Python arms hold, 200,
# against 202 cases on the day this was written: a generation that emitted
# nothing would otherwise leave a denominator of whatever the build compiled,
# and a gate that compares a run to an empty set compares nothing.
sce_gate_step "listing the Rust W3C suite the build placed"
cargo test --release -p sce-rust-tests --test w3c -- --list >"$LOG/list.log" 2>&1 \
    || { cat "$LOG/list.log" >&2; sce_gate_fail "the Rust W3C suite did not build"; }
expected="$(grep -cE '^test_[0-9]+[a-z]?::test_[0-9]+[a-z]?: test$' "$LOG/list.log" || true)"
if (( expected < 200 )); then
    sce_gate_fail "the build placed only $expected W3C case(s) in the target w3c (expected at least 200) — the suite would cover less than its name claims"
fi

sce_gate_http_fixture_server

sce_gate_step "running the Rust conformance suite"

# The log is printed whole because the lane's summary step reads its
# `test result:` lines out of what this gate writes.
status=0
cargo test --release --no-fail-fast -p sce-rust-tests >"$LOG/rust.log" 2>&1 || status=$?
cat "$LOG/rust.log"

if (( status != 0 )); then
    grep -E '^(test .* \.\.\. FAILED|error(\[E[0-9]+\])?:)' "$LOG/rust.log" | head -n 40 >&2 || true
    sce_gate_fail "Rust conformance suite failed"
fi

# A test binary that compiled no cases, or a target cargo was never asked to
# build, exits 0 with no case lines at all, which reads as a pass in the
# status alone. Every case the build placed has to answer.
cases="$(grep -cE '^test test_[0-9]+[a-z]?::test_[0-9]+[a-z]? \.\.\. ok$' "$LOG/rust.log" || true)"
if (( cases < expected )); then
    tail -n 20 "$LOG/rust.log" >&2
    sce_gate_fail "only $cases of the $expected W3C case(s) the generator wrote passed — the suite covered less than its name claims"
fi

sce_gate_step "$cases Rust W3C case(s) passed, of $expected generated"
