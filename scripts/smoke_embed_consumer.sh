#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# smoke_embed_consumer.sh — drift guard for the embed/ vendor payload.
#
# Builds a minimal consumer project in a scratch directory that
# vendors a freshly-packaged embed/ via add_subdirectory() and invokes
# sce_add_state_machine() on a trivial SCXML. This exercises the code
# paths that caught the tc8-harness SCEClangFormat.cmake drift:
#
#   1. include(embed/SCECodegen.cmake)       → requires SCEClangFormat.cmake present
#   2. SCE_TEMPLATE_DIR auto-detection       → requires tools/codegen/templates/ present
#   3. sce_add_state_machine() end-to-end    → requires codegen + sce_base link
#
# A missing asset that slipped past sce_codegen_assets.cmake SSOT
# fails here loudly rather than at the next consumer integration.
#
# Intended for CI; locally useful before bumping the embed snapshot.
#
# Usage:
#   ./scripts/smoke_embed_consumer.sh
#
# Exits 0 on success, non-zero on any configure/build failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# This script is reached by the `embed-vendor` gate, so its build competes with
# whatever else a push is running. The one place that decides how much of the
# machine such a build may ask for.
source "${SCE_ROOT}/scripts/lib/sce_build_jobs.sh"

# Regenerate embed/ in an isolated scratch dir so a stale in-repo
# embed/ cannot mask missing-file bugs — we want to assert that
# package_embed.sh alone (SSOT-driven) produces a shippable tree.
SCRATCH="$(mktemp -d -t sce_embed_smoke.XXXXXX)"
trap 'rm -rf "${SCRATCH}"' EXIT

EMBED_FRESH="${SCRATCH}/embed"
CONSUMER_SRC="${SCRATCH}/consumer"
CONSUMER_BUILD="${SCRATCH}/consumer_build"

# Pin the generator to this tree's build. The consumer project is built
# from a scratch directory, so `SCEFindCodegen.cmake`'s in-tree lookup
# finds nothing and falls through to `find_program(sce-codegen)` — i.e.
# whatever binary happens to sit on the developer's PATH. That pairs an
# arbitrarily old generator with this tree's templates, and the mismatch
# reads as a template bug: a generator predating a filter fails with
# "unknown filter" on a template that is perfectly valid here. Resolving
# through the shared locator asserts what the gate is for — that the
# embed payload *this tree produces* is consumer-usable.
source "${SCE_ROOT}/scripts/lib/sce_codegen.sh"
SCE_CODEGEN_BIN="$(sce_codegen_require "${SCE_ROOT}")"

echo "[smoke] Regenerating embed payload into ${EMBED_FRESH}"
"${SCRIPT_DIR}/package_embed.sh" -o "${EMBED_FRESH}" >/dev/null

# ----------------------------------------------------------------------------
# File-manifest drift guard: SSOT-listed assets must physically exist in
# the fresh embed tree. A missing file here means package_embed.sh
# bypassed the SSOT for this asset.
# ----------------------------------------------------------------------------
echo "[smoke] Verifying SSOT-listed assets landed in embed/"
MISSING=()
for required in SCECodegen.cmake SCEClangFormat.cmake \
                tools/codegen/templates/state_machine.jinja2 \
                tools/codegen/default.clang-format; do
    if [ ! -e "${EMBED_FRESH}/${required}" ]; then
        MISSING+=("${required}")
    fi
done
if [ ${#MISSING[@]} -gt 0 ]; then
    echo "ERROR: embed payload is missing SSOT-listed asset(s):" >&2
    for m in "${MISSING[@]}"; do
        echo "  - ${m}" >&2
    done
    echo "Check sce/sce_codegen_assets.cmake and package_embed.sh parsing." >&2
    exit 1
fi

# ----------------------------------------------------------------------------
# Consumer smoke build: tiny CMake project that exercises the embed
# integration surface sce_add_state_machine() depends on.
# ----------------------------------------------------------------------------
echo "[smoke] Building minimal consumer at ${CONSUMER_SRC}"
mkdir -p "${CONSUMER_SRC}/third_party"
# Move the fresh embed into the consumer tree to mirror the real
# vendoring layout (third_party/sce/).
mv "${EMBED_FRESH}" "${CONSUMER_SRC}/third_party/sce"

cat > "${CONSUMER_SRC}/smoke.scxml" <<'SCXMLEOF'
<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <final id="s1"/>
</scxml>
SCXMLEOF

cat > "${CONSUMER_SRC}/main.cpp" <<'CPPEOF'
// Minimal smoke: instantiate the generated SM + query its initial state.
// A successful link proves sce_base and the generated header are wired up.
#include "smoke_sm.h"
int main() {
    SCE::Generated::smoke::smoke sm;
    auto initial = SCE::Generated::smoke::smokePolicy::initialState();
    return initial == SCE::Generated::smoke::State::S0 ? 0 : 1;
}
CPPEOF

cat > "${CONSUMER_SRC}/CMakeLists.txt" <<'CMAKEEOF'
cmake_minimum_required(VERSION 3.14)
project(sce_embed_smoke CXX)

# Vendor SCE via the embed payload.
add_subdirectory(third_party/sce)

# Pull in sce_add_state_machine() — exercises include() of
# SCEClangFormat.cmake and SCE_TEMPLATE_DIR auto-detection.
include(${CMAKE_CURRENT_SOURCE_DIR}/third_party/sce/SCECodegen.cmake)

add_executable(smoke_consumer main.cpp)
sce_add_state_machine(
    TARGET     smoke_consumer
    SCXML_FILE ${CMAKE_CURRENT_SOURCE_DIR}/smoke.scxml)
target_link_libraries(smoke_consumer PRIVATE sce_base)
CMAKEEOF

echo "[smoke] Configuring consumer"
cmake -S "${CONSUMER_SRC}" -B "${CONSUMER_BUILD}" \
      -DCMAKE_BUILD_TYPE=Release \
      -DSCE_CODEGEN="${SCE_CODEGEN_BIN}" \
      >"${SCRATCH}/configure.log" 2>&1 || {
    echo "ERROR: consumer configure failed. Log tail:" >&2
    tail -n 40 "${SCRATCH}/configure.log" >&2
    exit 1
}

echo "[smoke] Building consumer"
cmake --build "${CONSUMER_BUILD}" --target smoke_consumer \
      --parallel "$(sce_build_jobs_value)" \
      >"${SCRATCH}/build.log" 2>&1 || {
    echo "ERROR: consumer build failed. Log tail:" >&2
    tail -n 40 "${SCRATCH}/build.log" >&2
    exit 1
}

echo "[smoke] OK: embed payload is consumer-usable end-to-end."
