#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The C11 (MCU backend) conformance surface: 204 W3C cases plus integration
# and unit fixtures, registered under the `c11` ctest label.
#
# This backend is what `watching-zenoh` ships on an MCU, and until its CI lane
# existed nothing verified it but whoever happened to run ctest. The lane
# closed that; this gate closes the other half, because a lane still reports
# after the push rather than before it.
#
# The floor and the executed-vs-registered comparison are the lane's, kept
# because they answer the two ways a labelled ctest run reports green on
# nothing: a configure that stops registering the fixtures, and a run that
# covers a subset of what it registered.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_main_build_dir
BUILD_DIR="$SCE_MAIN_BUILD_DIR"

# A killed run leaves its log temporary behind forever; the gate that creates
# them is the one that clears them.
sce_prune_ctest_temporaries "$BUILD_DIR"

# `sce_c11_tests` aggregates every target the C directory defines, so a
# fixture added later is built without anyone updating a list — and building
# it rather than the default target skips the C++ engine, mesh, forge and the
# examples, none of which this backend links.
sce_gate_step "building sce_c11_tests"
sce_gate_build "$BUILD_DIR" --target sce_c11_tests \
    || sce_gate_fail "sce_c11_tests build"

registered="$(ctest --test-dir "$BUILD_DIR" -N -L c11 | grep -cE '^ *Test +#')"
sce_gate_step "registered c11 tests: $registered"
if (( registered < 200 )); then
    sce_gate_fail "only $registered c11 test(s) registered (expected at least 200) — the gate would report on a smaller set than its name claims"
fi

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

# `--no-tests=error` covers the zero case even if the floor above is ever
# loosened; the count comparison below covers a partial run.
ctest --test-dir "$BUILD_DIR" -L c11 \
      --output-on-failure --no-tests=error -j "$(sce_build_jobs)" 2>&1 | tee "$LOG/ctest.log"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "c11 ctest suite failed"

executed="$(grep -oE 'out of [0-9]+' "$LOG/ctest.log" | tail -1 | grep -oE '[0-9]+')"
if [[ "$executed" != "$registered" ]]; then
    sce_gate_fail "ran ${executed:-0} of $registered registered c11 test(s)"
fi

sce_gate_step "$executed c11 test(s) passed"
