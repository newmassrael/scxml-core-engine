# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C++ half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli --release -p sce-build
#   cmake -S sce-forge-runtime/cpp/tests/conformance -B build_conformance_cpp
#   cmake --build build_conformance_cpp
#   ctest --test-dir build_conformance_cpp --output-on-failure
#
# Invokes sce-codegen at configure time on each fixture SCXML and writes the
# generated C++ headers into ${CMAKE_CURRENT_BINARY_DIR}/generated/, which is
# added to the test target's include path. The test binary then links against
# sce_forge_runtime (header-only, INTERFACE library) and nlohmann::json
# (header-only, vendored under third_party) to compare generated output
# against tests/forge/conformance/numerical_reference.json.

cmake_minimum_required(VERSION 3.16)
project(sce_forge_numerical_conformance CXX)

# RFC §7 A6: §5.A algorithm kind joins the cpp conformance harness. The
# A3 cpp algorithm template lowers `bytes` parameters to
# `std::span<const std::uint8_t>` per RFC §5.J.5 (non-owning view, the
# C++ idiom that avoids the "no STL containers" rule's intent of
# forbidding owning collections in MCU-targeted emit). `std::span` is
# C++20, so the harness bumps from C++17 → C++20 to compile the
# algorithm fixtures alongside the existing 11 baseline kinds. The
# baseline kinds remain C++17-clean, so this is purely additive.
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/cpp/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/sce-forge-runtime/cpp/include")
set(NLOHMANN_JSON_INCLUDE_DIR "${REPO_ROOT}/third_party/nlohmann_json/include")

# Rebuild sce-codegen from the current sce-build sources when the Rust
# toolchain is available (local dev). Cargo's incremental build makes this
# a near-instant no-op when nothing has changed; the value is eliminating
# the stale-binary foot-gun where a schema change in conformance.rs would
# otherwise be silently ignored by a C++ test invoking the old binary.
#
# The `file(GLOB_RECURSE ... CONFIGURE_DEPENDS)` call tracks sce-build
# sources as configure-time inputs, so CMake re-runs configure — and thus
# re-executes the cargo build below — whenever any .rs file changes. In CI
# the conformance-cpp job downloads a pre-built artifact from the build-
# codegen job and has no Rust toolchain; `find_program` returns NOTFOUND
# and the configure-time rebuild is skipped, falling through to the
# existence check.
find_program(CARGO_EXECUTABLE cargo)
if(CARGO_EXECUTABLE)
    file(GLOB_RECURSE _sce_build_sources CONFIGURE_DEPENDS
        "${REPO_ROOT}/sce-build/src/*.rs"
        "${REPO_ROOT}/sce-build/Cargo.toml"
    )
    execute_process(
        COMMAND "${CARGO_EXECUTABLE}" build
                --bin sce-codegen
                --features cli
                --release
                -p sce-build
        WORKING_DIRECTORY "${REPO_ROOT}"
        RESULT_VARIABLE _cargo_rc
    )
    if(NOT _cargo_rc EQUAL 0)
        message(FATAL_ERROR
            "cargo build of sce-codegen failed (rc=${_cargo_rc})"
        )
    endif()
endif()

if(NOT EXISTS "${SCE_CODEGEN}")
    message(FATAL_ERROR
        "sce-codegen binary not found at ${SCE_CODEGEN}. "
        "Build it first: "
        "cargo build --bin sce-codegen --features cli --release -p sce-build"
    )
endif()

if(NOT EXISTS "${REFERENCE_JSON}")
    message(FATAL_ERROR "reference JSON not found at ${REFERENCE_JSON}")
endif()

if(NOT EXISTS "${NLOHMANN_JSON_INCLUDE_DIR}/nlohmann/json.hpp")
    message(FATAL_ERROR
        "nlohmann/json not found at ${NLOHMANN_JSON_INCLUDE_DIR}"
    )
endif()

# Pull the fixture catalog from sce-codegen itself in the cmake-list format
# (single semicolon-separated literal). The Rust binary already owns the
# manifest schema, so cmake never has to touch JSON or shell out to python.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
    OUTPUT_VARIABLE FIXTURES
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _manifest_rc
)
if(NOT _manifest_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures failed for ${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation.
set(GENERATED_HEADERS "")
foreach(fixture IN LISTS FIXTURES)
    set(out_header "${GENERATED_DIR}/${fixture}.h")
    set(scxml "${RESOURCE_DIR}/${fixture}.scxml")
    add_custom_command(
        OUTPUT "${out_header}"
        COMMAND "${SCE_CODEGEN}" generate "${scxml}"
                --language cpp
                --output-dir "${GENERATED_DIR}"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        COMMENT "Generating C++ fixture ${fixture}.h"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS "${out_header}")
endforeach()

# Step 2: render the test harness itself from the shared template. The
# scaffold pulls in per-kind fragments via dynamic Jinja2 include, so the
# DEPENDS list must enumerate every fragment as well — otherwise editing
# kinds/filter.cpp.jinja2 alone would silently leave a stale harness.
file(GLOB CONFORMANCE_KIND_TEMPLATES
    CONFIGURE_DEPENDS
    "${CONFORMANCE_TEMPLATE_DIR}/kinds/*.cpp.jinja2"
)
set(HARNESS_SRC "${GENERATED_DIR}/numerical_conformance_test.cpp")
add_custom_command(
    OUTPUT "${HARNESS_SRC}"
    COMMAND "${SCE_CODEGEN}" generate-conformance
            --language cpp
            --manifest "${FIXTURE_MANIFEST}"
            --output-dir "${GENERATED_DIR}"
    DEPENDS "${FIXTURE_MANIFEST}"
            "${CONFORMANCE_TEMPLATE_DIR}/harness.cpp.jinja2"
            ${CONFORMANCE_KIND_TEMPLATES}
            "${SCE_CODEGEN}"
    COMMENT "Generating C++ conformance harness from template"
    VERBATIM
)

add_custom_target(generate_forge_fixtures_cpp
    DEPENDS ${GENERATED_HEADERS} "${HARNESS_SRC}"
)

add_executable(numerical_conformance_test "${HARNESS_SRC}")
add_dependencies(numerical_conformance_test generate_forge_fixtures_cpp)

target_include_directories(numerical_conformance_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${NLOHMANN_JSON_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_definitions(numerical_conformance_test PRIVATE
    "REFERENCE_JSON_PATH=\"${REFERENCE_JSON}\""
)

target_compile_options(numerical_conformance_test PRIVATE
    -Wall -Wextra -Wpedantic
)

enable_testing()
add_test(NAME numerical_conformance COMMAND numerical_conformance_test)
