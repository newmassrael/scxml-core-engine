#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# C++ arm of the forge conformance suite (forge-conformance.yml).
#
# Uses a dedicated scratch build dir so it does not clobber a developer's
# main build tree. The codegen dependency is passed explicitly via
# -DSCE_CODEGEN so CMake cannot quietly pick up a stale binary — the same
# failure the embed consumer smoke hit when it fell through to PATH.
#
# The build dominates and is already paid, so the suite runs here too. This
# is NOT the W3C ctest suite, which is left to CI — it is the conformance
# fixture that was just built, and it runs in hundredths of a second.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

BUILD_DIR="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$BUILD_DIR'"

# Build configuration mirrors the lane, because a gate that builds a different
# binary is not checking the one CI ships a verdict on: RelWithDebInfo turns on
# optimisation and keeps assertions, and an unset CMAKE_BUILD_TYPE does
# neither. The generator is a speed choice rather than a semantic one, so Ninja
# is used when it is installed and the platform default otherwise — a developer
# without ninja still runs the gate, on the same build type.
GENERATOR=()
if command -v ninja >/dev/null 2>&1; then
    GENERATOR=(-G Ninja)
else
    sce_gate_step "ninja not installed; using the default generator (same build type)"
fi

cmake -S backends/cpp/forge-runtime/tests/conformance \
      -B "$BUILD_DIR" \
      ${GENERATOR+"${GENERATOR[@]}"} \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo \
      -DSCE_CODEGEN="$CODEGEN" \
      -Wno-dev >/dev/null \
    || sce_gate_fail "C++ forge conformance configure"
cmake --build "$BUILD_DIR" --parallel "$(nproc)" \
    || sce_gate_fail "C++ forge conformance build"
ctest --test-dir "$BUILD_DIR" --output-on-failure \
    || sce_gate_fail "C++ forge conformance"
