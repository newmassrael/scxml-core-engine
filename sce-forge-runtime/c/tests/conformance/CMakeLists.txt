# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C11 half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli --release -p sce-build
#   cmake -S sce-forge-runtime/c/tests/conformance -B build_conformance_c
#   cmake --build build_conformance_c
#   ctest --test-dir build_conformance_c --output-on-failure
#
# Invokes sce-codegen at configure time on each Phase-A fixture SCXML
# and writes the generated C headers into ${CMAKE_CURRENT_BINARY_DIR}/
# generated/, which is added to the test target's include path. The
# test binary then links against sce_forge_runtime_c (header-only,
# INTERFACE library — empty for Phase A) and runs against pre-baked
# oracle arrays emitted directly by the harness template (RFC §5.J.2
# F7: C has no zero-deps JSON parser surviving the R3 lock-in, so the
# oracle is materialised at codegen time, not parsed at runtime).

cmake_minimum_required(VERSION 3.16)
project(sce_forge_numerical_conformance_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/c/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/sce-forge-runtime/c/include")

# Rebuild sce-codegen from current sce-build sources when the Rust
# toolchain is available (local dev). Mirrors the cpp version's
# stale-binary safeguard exactly. CI without a Rust toolchain falls
# through to the existence check.
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

# RFC §5.J.2 §3: pull the C11 fixture catalog from sce-codegen itself.
# The Rust binary owns the manifest schema and the per-language `kind`
# gate (`c11_supported_kind`), so cmake never has to touch JSON or
# duplicate the phase-staging list — `--language c11` filters out any
# kind whose `kinds/<kind>.c.jinja2` fragment has not landed yet,
# matching the filter `generate-conformance --language c11` applies at
# harness render time. After Phase E-4 every forge kind has its
# fragment, so today the filter is a no-op identity over the manifest;
# it stays in the toolchain so a future kind addition forces an
# explicit decision in `c11_supported_kind` instead of silently being
# pulled into the harness.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language c11
    OUTPUT_VARIABLE C11_FIXTURES
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _manifest_rc
)
if(NOT _manifest_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --language c11 failed for "
        "${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation. Each fixture's SCXML is
# fed to sce-codegen --language c11 to produce a single header.
set(GENERATED_HEADERS "")
foreach(fixture IN LISTS C11_FIXTURES)
    set(out_header "${GENERATED_DIR}/${fixture}.h")
    set(scxml "${RESOURCE_DIR}/${fixture}.scxml")
    add_custom_command(
        OUTPUT "${out_header}"
        COMMAND "${SCE_CODEGEN}" generate "${scxml}"
                --language c11
                --output-dir "${GENERATED_DIR}"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        COMMENT "Generating C11 fixture ${fixture}.h"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS "${out_header}")
endforeach()

# Step 2: render the test harness from the shared template. The
# scaffold pulls in per-kind fragments via dynamic Jinja2 include, so
# the DEPENDS list enumerates every fragment as well — editing one of
# them alone would otherwise silently leave a stale harness.
file(GLOB CONFORMANCE_KIND_TEMPLATES
    CONFIGURE_DEPENDS
    "${CONFORMANCE_TEMPLATE_DIR}/kinds/*.c.jinja2"
)
set(HARNESS_SRC "${GENERATED_DIR}/numerical_conformance_test.c")
add_custom_command(
    OUTPUT "${HARNESS_SRC}"
    COMMAND "${SCE_CODEGEN}" generate-conformance
            --language c11
            --manifest "${FIXTURE_MANIFEST}"
            --output-dir "${GENERATED_DIR}"
    DEPENDS "${FIXTURE_MANIFEST}"
            "${REFERENCE_JSON}"
            "${CONFORMANCE_TEMPLATE_DIR}/harness.c.jinja2"
            ${CONFORMANCE_KIND_TEMPLATES}
            "${SCE_CODEGEN}"
    COMMENT "Generating C11 conformance harness from template"
    VERBATIM
)

add_custom_target(generate_forge_fixtures_c
    DEPENDS ${GENERATED_HEADERS} "${HARNESS_SRC}"
)

add_executable(numerical_conformance_test_c "${HARNESS_SRC}")
add_dependencies(numerical_conformance_test_c generate_forge_fixtures_c)

target_include_directories(numerical_conformance_test_c PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

# RFC §5.J.2 §4 acceptance gate: -Werror with -Wall/-Wextra/-Wpedantic
# is the bar — Phase A's emitted code must survive without warnings.
target_compile_options(numerical_conformance_test_c PRIVATE
    -Wall -Wextra -Wpedantic -Werror
)

# math.h is included by the harness for fabs-style helpers; on most
# Unix toolchains math is in libc, but glibc requires explicit -lm.
target_link_libraries(numerical_conformance_test_c PRIVATE m)

enable_testing()
add_test(NAME numerical_conformance_c COMMAND numerical_conformance_test_c)
