# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity (cpp half) — runtime round-trip
# property test mirroring backends/rust/forge-conformance/tests/forge_default_round_trip.rs.
#
# Standalone CMake project that:
#   - regenerates sce-codegen if cargo is available
#   - generates the 3 default-marker fixtures into ${CMAKE_CURRENT_BINARY_DIR}/generated/
#   - builds a small test binary that asserts
#       T{}.encode_to_vec() decodes back into the declared default arm with cursor.remaining() == 0
#       and re-encoding produces byte-equal output
#
# Configure + build + test:
#   cargo build --bin sce-codegen --features cli -p sce-build
#   cmake -S backends/cpp/forge-runtime/tests/round_trip -B build_round_trip_cpp
#   cmake --build build_round_trip_cpp
#   ctest --test-dir build_round_trip_cpp --output-on-failure

cmake_minimum_required(VERSION 3.20)
project(sce_forge_round_trip_cpp CXX)

# C++17: the standard a consumer compiles the generated C++ at (docs/adr/0059,
# section 1). The codec's decode path initializes its struct by position, in
# declaration order, so it needs no C++20 designated initializer.
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/cpp/forge-runtime/include")

# Rebuild sce-codegen from current sources when cargo is available — same
# pattern as the numerical conformance harness. CI without a Rust toolchain
# falls through to the existence check.
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
        message(FATAL_ERROR "cargo build of sce-codegen failed (rc=${_cargo_rc})")
    endif()
endif()

# Every generation is declared from the generator's plan of it
# (`sce_codegen_generation`, cmake/SCECodegenPlan.cmake), as the tree
# declares its own: the files a fixture's generation writes, its depfile and
# its one command are the plan's answer, and nothing about them is written
# here. The module resolves the generator through cmake/SCEFindCodegen.cmake,
# which every harness finds it by and which aborts with the build command
# when no profile holds a binary.
include("${REPO_ROOT}/cmake/SCECodegenPlan.cmake")

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# The 3 RFC-dedicated default-marker fixtures. Lives outside the numerical
# conformance manifest because their purpose is contract testing (Default
# emission), not oracle comparison.
set(FIXTURES
    codec_default_marker_arm_a
    codec_default_marker_arm_b
    codec_variant_default_marker
    codec_embed_variant_dispatch
)

# The fixtures are generated into one directory, so they claim in one list: a
# file two of their generations would write is the first's.
set(GENERATED_HEADERS "")
foreach(fixture IN LISTS FIXTURES)
    sce_codegen_generation(_FIXTURE
        DOCUMENT "${RESOURCE_DIR}/${fixture}.scxml"
        OUTPUT_DIR "${GENERATED_DIR}"
        LANGUAGE cpp
        CLAIM SCE_ROUND_TRIP_CPP_DECLARED_OUTPUTS
        COMMENT "Generating round-trip fixture ${fixture} (cpp)")
    list(APPEND GENERATED_HEADERS ${_FIXTURE_HEADERS})
endforeach()

# The headers are the program's sources, so it waits for their generations.
add_executable(default_round_trip_test default_round_trip_test.cpp ${GENERATED_HEADERS})

target_include_directories(default_round_trip_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_options(default_round_trip_test PRIVATE
    -Wall -Wextra -Wpedantic
)

enable_testing()
add_test(NAME default_round_trip COMMAND default_round_trip_test)
