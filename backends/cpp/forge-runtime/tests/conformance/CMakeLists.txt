# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness (C++ half).
#
# Standalone CMake project. Configure and build with:
#
#   cargo build --bin sce-codegen --features cli --release -p sce-build
#   cmake -S backends/cpp/forge-runtime/tests/conformance -B build_conformance_cpp
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

# RFC §synth-7 A6: §synth-5-A algorithm kind joins the cpp conformance harness. The
# A3 cpp algorithm template lowers `bytes` parameters to
# `std::span<const std::uint8_t>` per RFC §synth-5-J-5 (non-owning view, the
# C++ idiom that avoids the "no STL containers" rule's intent of
# forbidding owning collections in MCU-targeted emit). `std::span` is
# C++20, so the harness bumps from C++17 → C++20 to compile the
# algorithm fixtures alongside the existing 11 baseline kinds. The
# baseline kinds remain C++17-clean, so this is purely additive.
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(FIXTURE_MANIFEST "${REPO_ROOT}/tests/forge/conformance/fixtures.json")
set(CONFORMANCE_TEMPLATE_DIR "${REPO_ROOT}/tools/codegen/templates/forge/cpp/conformance")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/cpp/forge-runtime/include")
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
#
# RFC §synth-5-B B2-test-vector: passing `--language cpp` activates the
# content-aware filter (`lang_supports_fixture`) so fixtures whose
# SCXML carries `<sce:test-vector>` are dropped on backends that have
# not yet shipped the sidecar emitter. Each closure widens the
# supported set as it lands; until then the cpp harness skips the
# gated fixture instead of surfacing a hard codegen error.
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language cpp
    OUTPUT_VARIABLE FIXTURES
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _manifest_rc
)
if(NOT _manifest_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --language cpp failed for ${FIXTURE_MANIFEST}")
endif()

# RFC §synth-5-B B2-test-vector: ask sce-codegen for the subset of C++
# fixtures whose SCXML carries `<sce:test-vector>` rows. Each such
# fixture emits `<fixture>_test.h` alongside `<fixture>.h`, so the
# per-fixture custom_command must declare both as OUTPUT for cmake's
# dependency tracker to pick them up. The harness template includes
# the sidecar conditionally on `f.has_test_vectors` and folds the
# returned failure count into `g_failures` from main().
execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${FIXTURE_MANIFEST}"
            --format cmake
            --language cpp
            --has-test-vectors
            --resource-dir "${RESOURCE_DIR}"
    OUTPUT_VARIABLE CPP_FIXTURES_WITH_TV
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _tv_rc
)
if(NOT _tv_rc EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures --has-test-vectors --language cpp failed for "
        "${FIXTURE_MANIFEST}")
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# Step 1: per-fixture product code generation. Fixtures whose SCXML
# carries `<sce:test-vector>` rows emit a sibling `<fixture>_test.h`
# sidecar; the OUTPUT list is widened so cmake's dependency tracker
# rebuilds the harness when either file moves.
set(GENERATED_HEADERS "")
foreach(fixture IN LISTS FIXTURES)
    set(out_header "${GENERATED_DIR}/${fixture}.h")
    set(scxml "${RESOURCE_DIR}/${fixture}.scxml")
    set(_outputs "${out_header}")
    list(FIND CPP_FIXTURES_WITH_TV "${fixture}" _tv_idx)
    if(NOT _tv_idx EQUAL -1)
        set(_sidecar "${GENERATED_DIR}/${fixture}_test.h")
        list(APPEND _outputs "${_sidecar}")
    endif()
    add_custom_command(
        OUTPUT ${_outputs}
        COMMAND "${SCE_CODEGEN}" generate "${scxml}"
                --language cpp
                --output-dir "${GENERATED_DIR}"
                --write-deps "${GENERATED_DIR}/${fixture}.h.d"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        DEPFILE "${GENERATED_DIR}/${fixture}.h.d"
        COMMENT "Generating C++ fixture ${fixture}.h"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS ${_outputs})
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
    # Promote three codegen-quality classes from warnings to errors so
    # template drift cannot ship silently broken generated headers:
    #   • narrowing — multi-byte BE/LE field unpack expressions must
    #     wrap in static_cast<carrier_t>(...) (streaming_fixed_field_body
    #     in sce-build/src/forge/generator.rs).
    #   • unused-parameter — throwing default-impl lambdas in procedure
    #     templates must use unnamed params (gcc does not honour
    #     Rust-style underscore-prefix suppression here).
    #   • type-limits — validator range checks must elide
    #     `unsigned < 0` and `T > T_MAX` via `is_unsigned` /
    #     `is_max_at_type_max` flags from range_rules data model.
    -Werror=narrowing
    -Werror=unused-parameter
    -Werror=type-limits
)

enable_testing()
add_test(NAME numerical_conformance COMMAND numerical_conformance_test)
