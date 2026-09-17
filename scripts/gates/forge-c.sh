#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# C11 arm of the forge conformance suite.
#
# ⚠ THIS ARM WAS NEVER RUN BY ANYTHING. Measured 2026-09-17: the harness
# template, the per-kind fragments (ten of them) and a complete standalone
# CMake project all existed under backends/c/forge-runtime/tests/conformance,
# and no gate script and no CI job invoked any of it. A suite nobody runs is
# indistinguishable from a suite that passes, which is how it stayed that way
# — the four sibling lanes (cpp/rust/python/go) made the set look complete.
#
# The first run found it healthy: 12 fixtures across 12 kinds, all green.
# What it had NOT been surviving was change, which is the point of a lane.
#
# ⚠⚠ The C11 arm bakes its oracle from `fixtures.json` at codegen time (C has
# no zero-deps JSON parser inside this backend's dependency rule), whereas the
# other five read `numerical_reference.json` at run time. A mutation that only
# edits the reference therefore cannot reach this arm; move the manifest to
# test it. Drift between the two sources is caught by the other five, whose
# fragments assert the oracle's names against the manifest's.
#
# Build configuration mirrors the C++ lane for the same reason it does: a
# gate that builds a different binary is not checking the one CI ships a
# verdict on.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

BUILD_DIR="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$BUILD_DIR'"

GENERATOR=()
if command -v ninja >/dev/null 2>&1; then
    GENERATOR=(-G Ninja)
else
    sce_gate_step "ninja not installed; using the default generator (same build type)"
fi

cmake -S backends/c/forge-runtime/tests/conformance \
      -B "$BUILD_DIR" \
      ${GENERATOR+"${GENERATOR[@]}"} \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo \
      -DSCE_CODEGEN="$CODEGEN" \
      -Wno-dev >/dev/null \
    || sce_gate_fail "C11 forge conformance configure"
sce_gate_build "$BUILD_DIR" \
    || sce_gate_fail "C11 forge conformance build"
ctest --test-dir "$BUILD_DIR" --output-on-failure \
    || sce_gate_fail "C11 forge conformance"
