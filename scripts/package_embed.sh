#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# package_embed.sh — Generate a self-contained C++ embed package (sce_core + sce_base)
#
# Produces an embed/ directory that consumer projects can vendor directly.
# No Rust toolchain, no Git, no network required to build from the embed package.
#
# Usage:
#   ./scripts/package_embed.sh [-o OUTPUT_DIR] [--with-spdlog]
#   OUTPUT_DIR defaults to ./embed/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${SCE_ROOT}/embed"
WITH_SPDLOG=false

# Argument parsing
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --with-spdlog)
            WITH_SPDLOG=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [-o OUTPUT_DIR] [--with-spdlog]"
            echo "  -o DIR         Output directory (default: ./embed/)"
            echo "  --with-spdlog  Include bundled spdlog headers"
            exit 0
            ;;
        *)
            # Backward compat: treat positional arg as output dir
            OUTPUT_DIR="$1"
            shift
            ;;
    esac
done

echo "SCE Embed Packager"
echo "  Source:  ${SCE_ROOT}"
echo "  Output:  ${OUTPUT_DIR}"

# Clean previous output
rm -rf "${OUTPUT_DIR}"

# ============================================================================
# 1. Read SSOT: sce_base_sources.cmake defines all file lists
# ============================================================================
SOURCES_CMAKE="${SCE_ROOT}/sce/sce_base_sources.cmake"
if [ ! -f "${SOURCES_CMAKE}" ]; then
    echo "ERROR: ${SOURCES_CMAKE} not found" >&2
    exit 1
fi

# Parse SCE_BASE_INCLUDE_DIRS from cmake file
INCLUDE_DIRS=$(grep -oP '(?<=^    )\w+' "${SOURCES_CMAKE}" | tail -n +1)
# More robust: extract from the set(SCE_BASE_INCLUDE_DIRS ...) block
INCLUDE_DIRS=$(sed -n '/set(SCE_BASE_INCLUDE_DIRS/,/)/{ s/^[[:space:]]*//; /^set\|^)/d; p; }' "${SOURCES_CMAKE}")

# Parse all .cpp files from SCE_BASE_SOURCES and SCE_BASE_SPDLOG_SOURCES
SOURCE_FILES=$(grep '\.cpp' "${SOURCES_CMAKE}" | sed 's/^[[:space:]]*//' | grep -v '^#')

# ============================================================================
# 2. Headers — directories listed in SCE_BASE_INCLUDE_DIRS (SSOT)
# ============================================================================
echo "Copying headers..."
mkdir -p "${OUTPUT_DIR}/include"

# Root headers (types.h, SCXMLTypes.h, GuardUtils.h, etc.)
find "${SCE_ROOT}/sce/include" -maxdepth 1 -name "*.h" -exec cp {} "${OUTPUT_DIR}/include/" \;

# Required subdirectories from SSOT
for dir in ${INCLUDE_DIRS}; do
    if [ -d "${SCE_ROOT}/sce/include/${dir}" ]; then
        cp -r "${SCE_ROOT}/sce/include/${dir}" "${OUTPUT_DIR}/include/"
    else
        echo "WARNING: include/${dir}/ not found" >&2
    fi
done

# ============================================================================
# 3. Sources — files listed in SCE_BASE_SOURCES + SCE_BASE_SPDLOG_SOURCES (SSOT)
# ============================================================================
echo "Copying sce_base sources..."

for src in ${SOURCE_FILES}; do
    dir=$(dirname "${src}")
    mkdir -p "${OUTPUT_DIR}/${dir}"
    cp "${SCE_ROOT}/sce/${src}" "${OUTPUT_DIR}/${dir}/"
done

# sce_base_sources.cmake itself (needed by embed CMakeLists.txt)
cp "${SOURCES_CMAKE}" "${OUTPUT_DIR}/"

# ============================================================================
# 3. Third-party dependencies (header-only)
# ============================================================================
echo "Copying third-party headers..."

# nlohmann/json (single-include)
mkdir -p "${OUTPUT_DIR}/third_party/nlohmann_json/include/nlohmann"
cp "${SCE_ROOT}/third_party/nlohmann_json/include/nlohmann/json.hpp" \
   "${OUTPUT_DIR}/third_party/nlohmann_json/include/nlohmann/"

# pugixml (headers + implementation — needed by sce_base for XMLDOMWrapper)
mkdir -p "${OUTPUT_DIR}/third_party/pugixml/src"
cp "${SCE_ROOT}/third_party/pugixml/src/pugixml.hpp"    "${OUTPUT_DIR}/third_party/pugixml/src/"
cp "${SCE_ROOT}/third_party/pugixml/src/pugiconfig.hpp" "${OUTPUT_DIR}/third_party/pugixml/src/"
cp "${SCE_ROOT}/third_party/pugixml/src/pugixml.cpp"    "${OUTPUT_DIR}/third_party/pugixml/src/"

# spdlog (header-only, optional — only needed if consumer sets SCE_USE_SPDLOG=ON)
if ${WITH_SPDLOG} && [ -d "${SCE_ROOT}/third_party/spdlog/include" ]; then
    echo "Including bundled spdlog..."
    mkdir -p "${OUTPUT_DIR}/third_party/spdlog"
    cp -r "${SCE_ROOT}/third_party/spdlog/include" "${OUTPUT_DIR}/third_party/spdlog/"
fi

# ============================================================================
# 4. Build system files for consumer integration
# ============================================================================
echo "Generating build system files..."

# CMake
cp "${SCE_ROOT}/scripts/embed_CMakeLists.txt" "${OUTPUT_DIR}/CMakeLists.txt"

# Bazel — generate BUILD.bazel from sce_base_sources.cmake (SSOT)
# Parse SCE_BASE_SOURCES from cmake, excluding SpdlogBackend (conditional).
SRCS=$(grep '\.cpp' "${SOURCES_CMAKE}" \
    | sed 's/^[[:space:]]*//' \
    | grep -v '^#' \
    | grep -v 'SpdlogBackend')
SRCS_FORMATTED=""
for src in ${SRCS}; do
    SRCS_FORMATTED="${SRCS_FORMATTED}        \"${src}\",
"
done

cat > "${OUTPUT_DIR}/BUILD.bazel" <<BZLEOF
# SCE Embed Package — auto-generated by package_embed.sh
# Source list parsed from sce_base_sources.cmake (SSOT).

cc_library(
    name = "nlohmann_json",
    hdrs = ["third_party/nlohmann_json/include/nlohmann/json.hpp"],
    includes = ["third_party/nlohmann_json/include"],
    visibility = ["//visibility:private"],
)

cc_library(
    name = "pugixml",
    srcs = ["third_party/pugixml/src/pugixml.cpp"],
    hdrs = [
        "third_party/pugixml/src/pugiconfig.hpp",
        "third_party/pugixml/src/pugixml.hpp",
    ],
    includes = ["third_party/pugixml/src"],
    visibility = ["//visibility:private"],
)

cc_library(
    name = "sce_core",
    hdrs = glob(["include/**/*.h"]),
    includes = ["include"],
    deps = [
        ":nlohmann_json",
        ":pugixml",
    ],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "sce_base",
    srcs = [
${SRCS_FORMATTED}    ],
    copts = [
        "-include common/ExternTemplates.h",
        "-include common/DisableStdOut.h",
    ],
    local_defines = ["SCE_ENABLE_RUNTIME_LOGGING"],
    deps = [":sce_core"],
    visibility = ["//visibility:public"],
)
BZLEOF

# Bazel repository rule for consumers referencing SCE source tree directly
cp "${SCE_ROOT}/bazel/sce_repo.bzl" "${OUTPUT_DIR}/"

# SCECodegen.cmake — provides sce_add_state_machine() for build-time codegen
cp "${SCE_ROOT}/cmake/SCECodegen.cmake" "${OUTPUT_DIR}/"

# ============================================================================
# 5. VERSION file
# ============================================================================
if command -v git &>/dev/null && [ -d "${SCE_ROOT}/.git" ]; then
    VERSION="$(cd "${SCE_ROOT}" && git describe --tags --always 2>/dev/null || echo "unknown")"
else
    VERSION="unknown"
fi
echo "${VERSION}" > "${OUTPUT_DIR}/VERSION"

# ============================================================================
# 6. Public-header manifest — emit after all headers are in place so the
#    AST scan sees the exact surface consumers will receive. Skipped when
#    clang++-19 is absent (manifest is advisory; consumers vendoring
#    without clang can still use the embed package).
# ============================================================================
CLANG_FOR_MANIFEST="${CLANG:-clang++-19}"
if command -v "${CLANG_FOR_MANIFEST}" >/dev/null 2>&1; then
    echo "Emitting public-header manifest..."
    "${SCRIPT_DIR}/emit_embed_manifest.sh" -d "${OUTPUT_DIR}"
else
    echo "WARNING: ${CLANG_FOR_MANIFEST} not found; skipping MANIFEST.json emit." >&2
    echo "         Consumers that rely on the manifest should install clang-19." >&2
fi

# ============================================================================
# Summary
# ============================================================================
HEADER_COUNT=$(find "${OUTPUT_DIR}/include" -name "*.h" | wc -l)
SOURCE_COUNT=$(find "${OUTPUT_DIR}/src" -name "*.cpp" | wc -l)
echo ""
echo "Embed package generated:"
echo "  Headers:  ${HEADER_COUNT}"
echo "  Sources:  ${SOURCE_COUNT}"
echo "  Version:  ${VERSION}"
echo "  Output:   ${OUTPUT_DIR}/"
echo ""
echo "Consumer integration:"
echo "  add_subdirectory(third_party/sce)"
echo "  target_link_libraries(my_app PRIVATE sce_base)"
