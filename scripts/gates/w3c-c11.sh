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

BUILD_DIR="${SCE_W3C_BUILD_DIR:-build}"

if [[ -f "$BUILD_DIR/CMakeCache.txt" ]]; then
    configured="$(sed -n 's/^CMAKE_BUILD_TYPE:STRING=//p' "$BUILD_DIR/CMakeCache.txt")"
    if [[ "$configured" != "RelWithDebInfo" ]]; then
        sce_gate_fail "$BUILD_DIR is configured CMAKE_BUILD_TYPE=${configured:-<unset>}; the lane builds RelWithDebInfo. Reconfigure with: cmake -B $BUILD_DIR -DCMAKE_BUILD_TYPE=RelWithDebInfo -G Ninja"
    fi
else
    sce_gate_step "configuring $BUILD_DIR (RelWithDebInfo, mirroring the lane)"
    GENERATOR=()
    command -v ninja >/dev/null 2>&1 && GENERATOR=(-G Ninja)
    cmake -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=RelWithDebInfo \
          ${GENERATOR+"${GENERATOR[@]}"} -Wno-dev >/dev/null \
        || sce_gate_fail "cmake configure"
fi

# `sce_c11_tests` aggregates every target the C directory defines, so a
# fixture added later is built without anyone updating a list — and building
# it rather than the default target skips the C++ engine, mesh, forge and the
# examples, none of which this backend links.
sce_gate_step "building sce_c11_tests"
cmake --build "$BUILD_DIR" --target sce_c11_tests --parallel "$(nproc)" >/dev/null \
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
      --output-on-failure --no-tests=error -j "$(nproc)" 2>&1 | tee "$LOG/ctest.log"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "c11 ctest suite failed"

executed="$(grep -oE 'out of [0-9]+' "$LOG/ctest.log" | tail -1 | grep -oE '[0-9]+')"
if [[ "$executed" != "$registered" ]]; then
    sce_gate_fail "ran ${executed:-0} of $registered registered c11 test(s)"
fi

sce_gate_step "$executed c11 test(s) passed"
