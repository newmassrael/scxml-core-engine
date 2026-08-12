#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: cpp-suite.yml
#
# Every ctest case in the main build that the C11 arm does not run: the C++
# engine's own unit suites, the mesh transports, and the cross-device
# conformance drivers.
#
# Measured 2026-08-12, and the measurement is why this exists. `scripts/gate
# --all` was run with a logging shim in place of `ctest`, so every invocation
# recorded its working directory and its arguments. Twenty-eight gates passed
# and exactly three ctest runs happened: two from `w3c-c11` (`-N -L c11` then
# `-L c11`) and one from `forge-cpp` against its own scratch tree. Three of
# these binaries were additionally replaced with loggers that exec the real
# one, and none was executed. So of 382 registered cases, 223 ran and 159 —
# 114 of them `mesh_*`, including all 19 zenoh cases, plus twenty-four C++ unit
# suites — were run by nothing at all. Not by a gate, and not by CI either: no
# workflow configures the main tree, and the C++ conformance job builds the
# single `w3c_test_cli` target.
#
# `-L c11` and `-LE c11` are exact complements, so the pair of gates covers the
# registered set with no third list to keep current — that partition is
# asserted below and pinned by `sce-build/tests/ctest_lane_partition.rs`. The
# W3C C++ cases are not in it: they are not ctest cases at all, they are the
# 404 the `w3c-cpp` binary runs, so nothing here is a second spelling of a run
# another gate already makes.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# A live fixture server makes 13 of these cases report Not Run, which is a
# smaller suite reported as a passing one. The count comparison at the end
# would catch it; this names the cause instead of the symptom.
sce_gate_requires_free_http_port

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# The default target, not a named one. These cases span the engine, the mesh
# transports and the fixtures, and a target list here would be the same
# list-shaped defect this gate exists to remove: a case whose target nobody
# added would be registered, skipped by the build, and then reported by ctest
# as a missing executable rather than as an untested one.
sce_gate_step "building the main tree"
cmake --build "$BUILD_DIR" --parallel "$(nproc)" >/dev/null \
    || sce_gate_fail "main tree build"

count_registered() {
    ctest --test-dir "$BUILD_DIR" -N "$@" | grep -cE '^ *Test +#'
}

registered="$(count_registered -LE c11)"
c11="$(count_registered -L c11)"
total="$(count_registered)"

# The partition is the whole argument for selecting by label rather than by
# name. If the two halves ever stop adding up to the whole, a case belongs to
# neither gate and is back where the 159 were.
if (( registered + c11 != total )); then
    sce_gate_fail "the ctest partition is not total: $registered non-c11 + $c11 c11 != $total registered. A case in neither half is run by no gate — which is the state this gate was added to end."
fi

sce_gate_step "registered non-c11 tests: $registered (of $total)"
if (( registered < 140 )); then
    sce_gate_fail "only $registered non-c11 test(s) registered (expected at least 140) — the gate would report on a smaller set than its name claims"
fi

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# `--no-tests=error` covers the zero case even if the floor above is ever
# loosened; the count comparison below covers a partial run.
ctest --test-dir "$BUILD_DIR" -LE c11 \
      --output-on-failure --no-tests=error -j "$(nproc)" 2>&1 | tee "$LOG/ctest.log"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "C++ ctest suite failed"

executed="$(grep -oE 'out of [0-9]+' "$LOG/ctest.log" | tail -1 | grep -oE '[0-9]+')"
if [[ "$executed" != "$registered" ]]; then
    sce_gate_fail "ran ${executed:-0} of $registered registered non-c11 test(s)"
fi

sce_gate_step "$executed C++ test(s) passed"
