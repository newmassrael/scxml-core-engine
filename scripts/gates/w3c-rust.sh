#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Rust AOT conformance arm: the W3C cases, each its own integration test
# binary under `backends/rust/tests/tests/`, plus the hand-written tests beside
# them in `sce-rust-tests`.
#
# It was the one W3C arm with no gate. The `test-rust` job spelled its
# generation, its fixture server and its `cargo test` inline, while the Go,
# Python, Kotlin, C++ and C11 jobs each run `scripts/gate w3c-<backend>`. A
# backend's lanes are read from the gate registry by the rule
# `<family>-<canonical name>`, so a lane with no gate was a row nobody could
# derive. The job runs this script now, and the commands live here once.
#
# The Rust W3C tree is COMMITTED, unlike Go's, so the generation below
# rewrites tracked files. That is what the lane always did: it judges the
# generator in the working tree, not an artifact produced earlier. Whether the
# committed tree is what the generator writes is `regen-reproduces`'s
# question, not this gate's.
#
# `--release` and `--no-fail-fast`, as the lane ran it. The same cases also run
# in the test profile inside `workspace-tests`, over the committed tree and
# without regenerating it; this is the arm that runs what the generator writes
# now, optimised, the way the crate's own documentation runs it.
#
# The BasicHTTP fixtures need the echo server, and the harness addresses
# whatever port `SCE_W3C_HTTP_PORT` names, which the fixture helper exports
# once the server reports itself ready. The lane used to start it with a
# backgrounded `node` and a one-second sleep, the timer `lib.sh` records as
# answering before the port was bound.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# No toolchain check of its own: cargo is what `sce_gate_codegen` builds the
# generator with, so a machine without it stops on the next line, saying so.
CODEGEN="$(sce_gate_codegen)"

sce_gate_step "generating the Rust W3C suite"
"$CODEGEN" generate-w3c -l rust >/dev/null \
    || sce_gate_fail "Rust W3C generation"

# The population the run is held to, read from what generation left rather
# than written here. Each W3C case is one file holding one `#[test]` named
# after it (test_403a.rs holds `fn test_403a`), so the files are the
# denominator and the passing case lines are the numerator.
#
# The floor under the denominator is the one the Go and Python arms hold, 200,
# against 202 files in the tree on the day this was written: a generation that
# emitted nothing would otherwise leave a denominator of whatever happened to
# be on disk, and a gate that compares a run to an empty set compares nothing.
shopt -s nullglob
W3C_CASES=(backends/rust/tests/tests/test_[0-9]*.rs)
shopt -u nullglob
expected=${#W3C_CASES[@]}
if (( expected < 200 )); then
    sce_gate_fail "generation left only $expected W3C test file(s) under backends/rust/tests/tests (expected at least 200) — the suite would cover less than its name claims"
fi

sce_gate_http_fixture_server

sce_gate_step "running the Rust conformance suite"
LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

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
# status alone. Every file the generator wrote has to answer.
cases="$(grep -cE '^test test_[0-9]+[a-z]? \.\.\. ok$' "$LOG/rust.log" || true)"
if (( cases < expected )); then
    tail -n 20 "$LOG/rust.log" >&2
    sce_gate_fail "only $cases of the $expected W3C case(s) the generator wrote passed — the suite covered less than its name claims"
fi

sce_gate_step "$cases Rust W3C case(s) passed, of $expected generated"
