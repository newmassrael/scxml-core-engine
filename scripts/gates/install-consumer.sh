#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: install-consumer.yml
#
# An installed SCE builds what its own generator writes.
#
# The install exports `sce-codegen`, its templates and `SCECodegen.cmake`, so a
# consumer of an installed tree generates C++ with it, and that C++ includes the
# Forge C++ runtime (`sce/forge/algorithm.h`, `optional.h`, `span.h`). Until
# that runtime was an exported target of the main project, the install shipped
# one of its headers and only under SCE_ENABLE_MESH, no target carried its
# include directory, and the test that would have said so was opt-in and run by
# no lane: an installed SCE could not compile the C++ it generated, and nothing
# reported it.
#
# The measurement, and why it has this shape:
#
#   1. A scratch configure of the main project with tests and examples off and
#      the mesh at its default (off), because generated statechart code needs
#      the Forge runtime without the mesh, and a configure that turned the mesh
#      on would hide a rule that installs the runtime only for it. Its own
#      directory, because the install is of a built tree and a developer's
#      `build/` is theirs.
#   2. The build: `cmake --install` installs built artifacts, so every
#      installable target is built first.
#   3. `tests/install_tree_consumer/run.sh` against that tree: install to a
#      scratch prefix, then four probe projects that find the package there.
#      Three include each tier's headers; the fourth generates a statechart
#      with the INSTALLED SCECodegen.cmake and sce-codegen, links SCE::sce_base
#      alone, compiles at C++17 with warnings as errors, and runs.
#
# The generator formats the C++ it writes with clang-format of major 19 and
# refuses without it (docs/SCE_CODEGEN_DETERMINISM.md section 9); the installed
# generator in the fourth probe does the same, so the workflow installs it.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)" \
    || sce_gate_fail "could not provide sce-codegen, which the configure needs and the install ships"

WORK="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$WORK'"
# The root CMakeLists refuses a binary directory not named `build` or `build_*`.
BUILD_DIR="$WORK/build_install_consumer"

GENERATOR=()
command -v ninja >/dev/null 2>&1 && GENERATOR=(-G Ninja)

sce_gate_step "configuring $BUILD_DIR (no tests, no examples, mesh at its default)"
if ! cmake -S . -B "$BUILD_DIR" ${GENERATOR+"${GENERATOR[@]}"} \
        -DCMAKE_BUILD_TYPE=Release \
        -DBUILD_TESTS=OFF -DBUILD_EXAMPLES=OFF \
        -DSCE_CODEGEN="$CODEGEN" \
        -Wno-dev >"$WORK/configure.log" 2>&1; then
    tail -60 "$WORK/configure.log" >&2
    sce_gate_fail "configure"
fi

sce_gate_step "building what the install ships"
sce_gate_build "$BUILD_DIR" \
    || sce_gate_fail "build of the installable targets"

sce_gate_step "installing to a scratch prefix and building the consumer probes against it"
bash tests/install_tree_consumer/run.sh "$SCE_REPO_ROOT" "$BUILD_DIR" \
    || sce_gate_fail "an installed SCE does not serve its consumer (the failing probe and its output are above)"
