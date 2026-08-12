#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Go AOT conformance arm: 202 W3C cases plus the integration fixtures.
#
# The generated suite is not committed — `generate-w3c -l go` writes it into a
# gitignored tree — so this gate regenerates before running, exactly as the
# lane does. That also means the gate judges the generator in the working
# tree rather than an artifact somebody produced earlier.
#
# The BasicHTTP fixtures need the echo server: without it the suite reports 25
# failures that say nothing about the backend, measured on 2026-08-11 while
# this gate was being written.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Not skip-capable, unlike the clang-dependent gates. `sce_gate_requires_tool`
# exists so a developer missing a heavy toolchain still gets the rest of the
# suite, and it pairs with a lane that installs the package and sets
# SCE_REQUIRE_TOOLS. Neither half fits here: the lane obtains Go through
# `actions/setup-go` rather than a package install, and this gate is only
# selected when the Go backend changed — a skip there is silence about the
# exact change that asked for the check.
command -v go >/dev/null 2>&1 \
    || sce_gate_fail "go is not on PATH, and this gate was selected because the Go backend changed. Install Go (apt install golang, or https://go.dev/dl/) — skipping here would report green on an unverified backend."

CODEGEN="$(sce_gate_codegen)"

# Pin `generated-at` the way `scripts/regen_all_committed_trees.sh` does.
# Without this the generation stamps a fresh wall-clock value into every file
# under `backends/go/tests/generated`, which `committed_trees_carry_a_pinned_
# generated_at` then fails on — measured: this gate ran first, and the
# workspace suite three gates later reported 451 unpinned files. A gate that
# breaks the next gate is worse than no gate.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

sce_gate_step "generating the Go W3C suite"
"$CODEGEN" generate-w3c -l go >/dev/null \
    || sce_gate_fail "Go W3C generation"

sce_gate_http_fixture_server

sce_gate_step "running the Go conformance suite"
LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# `-v` matches the lane, whose summary step counts the per-case `--- PASS:`
# lines out of this log. It also gives this gate a case count rather than a
# package count to hold a floor against.
status=0
( cd backends/go/tests && go test ./... -v -count=1 ) >"$LOG/go.log" 2>&1 || status=$?
cat "$LOG/go.log"

if (( status != 0 )); then
    grep -E '^(--- FAIL|FAIL)' "$LOG/go.log" | head -n 20 >&2
    sce_gate_fail "Go conformance suite failed"
fi

# A suite that compiles nothing exits 0 with no case lines at all, which reads
# as a pass in the status alone. The floor is 200 against a current 215.
cases="$(grep -cE '^--- (PASS|FAIL):' "$LOG/go.log" || true)"
if (( cases < 200 )); then
    tail -n 20 "$LOG/go.log" >&2
    sce_gate_fail "only $cases Go case(s) ran (expected at least 200) — the suite covered less than its name claims"
fi

sce_gate_step "$cases Go case(s) passed"

# The same generator, asked for a module of the caller's own.
#
# `--output-dir` used to emit a fragment: the generated tests imported
# `github.com/newmassrael/sce-go-tests/harness` by literal, and neither go.mod
# nor the harness itself was written, so the tree compiled only inside a
# checkout carrying this repository's module path. The Rust half of that claim
# is gated inside `cargo test`
# (`sce-build/tests/conformance_suite_standalone.rs`); the Go half is here,
# because this is where the Go toolchain is known to exist.
#
# One fixture, not 202: the claim is about packaging, and a module with one
# package exercises go.mod, go.sum and the harness import exactly as a full one
# does.
sce_gate_step "building an emitted Go suite under a module name of its own"
SUITE="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$SUITE'"

"$CODEGEN" generate-w3c -l go \
    --output-dir "$SUITE" \
    --suite-package github.com/sce-gate/conformance \
    -t 144 >/dev/null \
    || sce_gate_fail "emitting a standalone Go suite"

suite_status=0
( cd "$SUITE/backends/go/tests" && go test ./... -v -count=1 ) \
    >"$LOG/go-suite.log" 2>&1 || suite_status=$?
if (( suite_status != 0 )); then
    cat "$LOG/go-suite.log" >&2
    sce_gate_fail "an emitted Go suite must build and pass from its own tree — that is what --output-dir claims"
fi

# Same reason as the floor above, and more load-bearing here: this tree holds
# one case, so "compiled nothing" and "ran everything" differ by a single line.
suite_cases="$(grep -cE '^--- (PASS|FAIL):' "$LOG/go-suite.log" || true)"
if (( suite_cases < 1 )); then
    cat "$LOG/go-suite.log" >&2
    sce_gate_fail "the emitted Go suite ran no cases — it compiled without collecting its fixture"
fi

# The import the whole exercise is about. Asserted on the source rather than
# inferred from the build, because a module that happened to be named like this
# repository's would compile either way.
if grep -q 'newmassrael/sce-go-tests' "$SUITE/backends/go/tests/generated/test144/test144_test.go"; then
    sce_gate_fail "the emitted Go test still imports this repository's own module"
fi

sce_gate_step "the emitted Go suite built and passed $suite_cases case(s)"
