# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C11 half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli -p sce-build
#   cmake -S backends/c/forge-runtime/tests/conformance -B build_conformance_c
#   cmake --build build_conformance_c
#   ctest --test-dir build_conformance_c --output-on-failure
#
# Invokes sce-codegen at configure time on each fixture SCXML
# and writes the generated C headers into ${CMAKE_CURRENT_BINARY_DIR}/
# generated/, which is added to the test target's include path. The
# test binary then links against sce_forge_runtime_c (header-only
# INTERFACE library) and runs against pre-baked
# oracle arrays emitted directly by the harness template (C has no
# zero-deps JSON parser within the backend's zero-dependency rule, so
# the oracle is materialised at codegen time, not parsed at runtime).

cmake_minimum_required(VERSION 3.16)
project(sce_forge_numerical_conformance_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/c/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/forge-runtime/include")
# Tier 1 header tree. Generated forge C headers reach into it for
# `<sce/sample.h>` (ownership contract) and `<sce/portability.h>`
# (`SCE_STATIC_ASSERT`); see backends/c/forge-runtime/CMakeLists.txt.
set(CORE_RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/runtime/include")

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

# Resolve the generator. The search lives in cmake/SCEFindCodegen.cmake
# so every harness finds it the same way and none of them names a
# build profile; it aborts with the build command when no profile
# holds a binary, which is what the hand-written check here used to do.
include("${REPO_ROOT}/cmake/SCEFindCodegen.cmake")

if(NOT EXISTS "${REFERENCE_JSON}")
    message(FATAL_ERROR "reference JSON not found at ${REFERENCE_JSON}")
endif()

# Pull the C11 fixture catalog from sce-codegen itself.
# The Rust binary owns the manifest schema and the per-language `kind`
# gate (`c11_supported_kind`), so cmake never has to touch JSON or
# duplicate the kind list — `--language c11` filters to kinds
# whose `kinds/<kind>.c.jinja2` fragment exists,
# matching the filter `generate-conformance --language c11` applies at
# harness render time. Every forge kind has its
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

# RFC §synth-5-B B2-test-vector: ask sce-codegen for the subset of C11
# fixtures whose SCXML carries `<sce:test-vector>` rows. Each such
# fixture emits `<fixture>_test.h` alongside `<fixture>.h`, so the
# per-fixture custom_command must declare both as OUTPUT for cmake's
# dependency tracker to pick them up. The harness template includes
# the sidecar conditionally on `f.has_test_vectors`.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language c11
            --has-test-vectors
            --resource-dir "${RESOURCE_DIR}"
    OUTPUT_VARIABLE C11_FIXTURES_WITH_TV
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _tv_rc
)
if(NOT _tv_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --has-test-vectors --language c11 failed for "
        "${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation. Each fixture's SCXML is
# fed to sce-codegen --language c11 to produce a single header. RFC §synth-5-B
# B2-test-vector: fixtures whose SCXML carries `<sce:test-vector>` rows
# emit a sibling `<fixture>_test.h` sidecar; the OUTPUT list is widened
# so cmake's dependency tracker rebuilds the harness when either file
# moves.
set(GENERATED_HEADERS "")
foreach(fixture IN LISTS C11_FIXTURES)
    set(out_header "${GENERATED_DIR}/${fixture}.h")
    set(scxml "${RESOURCE_DIR}/${fixture}.scxml")
    set(_outputs "${out_header}")
    list(FIND C11_FIXTURES_WITH_TV "${fixture}" _tv_idx)
    if(NOT _tv_idx EQUAL -1)
        set(_sidecar "${GENERATED_DIR}/${fixture}_test.h")
        list(APPEND _outputs "${_sidecar}")
    endif()
    # DEPFILE carries the ~120 jinja2 templates that rendered the output;
    # `DEPENDS` alone names only the SCXML, so a template edit would
    # leave the fixture stale and the build would reuse it silently.
    add_custom_command(
        OUTPUT ${_outputs}
        COMMAND "${SCE_CODEGEN}" generate "${scxml}"
                --language c11
                --output-dir "${GENERATED_DIR}"
                --write-deps "${GENERATED_DIR}/${fixture}.h.d"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        DEPFILE "${GENERATED_DIR}/${fixture}.h.d"
        COMMENT "Generating C11 fixture ${fixture}.h"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS ${_outputs})
endforeach()

# Step 2: render the test harness from the shared template. The
# scaffold pulls in per-kind fragments via dynamic Jinja2 include, and
# folds every fixture document under tests/forge/resources into the
# harness `source-hash` — so the generator states its own prerequisites
# through a DEPFILE, as every other codegen step in the tree does. The
# `file(GLOB ... CONFIGURE_DEPENDS)` this replaced named the fragments
# the scaffold includes directly and nothing they pull in, and named no
# fixture document at all.
set(HARNESS_SRC "${GENERATED_DIR}/numerical_conformance_test.c")
add_custom_command(
    OUTPUT "${HARNESS_SRC}"
    COMMAND "${SCE_CODEGEN}" generate-conformance
            --language c11
            --manifest "${FIXTURE_MANIFEST}"
            --output-dir "${GENERATED_DIR}"
            --write-deps "${HARNESS_SRC}.d"
    DEPENDS "${FIXTURE_MANIFEST}"
            "${REFERENCE_JSON}"
            "${SCE_CODEGEN}"
    DEPFILE "${HARNESS_SRC}.d"
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
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

# Acceptance gate: -Werror with -Wall/-Wextra/-Wpedantic
# is the bar — emitted code must survive without warnings.
target_compile_options(numerical_conformance_test_c PRIVATE
    -Wall -Wextra -Wpedantic -Werror
)

# math.h is included by the harness for fabs-style helpers; on most
# Unix toolchains math is in libc, but glibc requires explicit -lm.
target_link_libraries(numerical_conformance_test_c PRIVATE m)

enable_testing()
add_test(NAME numerical_conformance_c COMMAND numerical_conformance_test_c)
