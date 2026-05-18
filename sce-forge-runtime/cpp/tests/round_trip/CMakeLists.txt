# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity Atomic γ-3 (cpp half) — runtime round-trip
# property test mirroring sce-forge-runtime/rust/tests/forge_default_round_trip.rs.
#
# Standalone CMake project that:
#   - regenerates sce-codegen if cargo is available
#   - generates the 3 default-marker fixtures into ${CMAKE_CURRENT_BINARY_DIR}/generated/
#   - builds a small test binary that asserts
#       T{}.encode() decodes back into the declared default arm with cursor.remaining() == 0
#       and re-encoding produces byte-equal output
#
# Configure + build + test:
#   cargo build --bin sce-codegen --features cli --release -p sce-build
#   cmake -S sce-forge-runtime/cpp/tests/round_trip -B build_round_trip_cpp
#   cmake --build build_round_trip_cpp
#   ctest --test-dir build_round_trip_cpp --output-on-failure

cmake_minimum_required(VERSION 3.16)
project(sce_forge_round_trip_cpp CXX)

# C++20 because the cpp codec emit uses designated initializers
# (`.field = value`) in the decode path — matches the parent
# conformance harness which already bumped from C++17 → C++20 to
# absorb `std::span` in the algorithm A3 template.
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/sce-forge-runtime/cpp/include")

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
                --release
                -p sce-build
        WORKING_DIRECTORY "${REPO_ROOT}"
        RESULT_VARIABLE _cargo_rc
    )
    if(NOT _cargo_rc EQUAL 0)
        message(FATAL_ERROR "cargo build of sce-codegen failed (rc=${_cargo_rc})")
    endif()
endif()

if(NOT EXISTS "${SCE_CODEGEN}")
    message(FATAL_ERROR
        "sce-codegen binary not found at ${SCE_CODEGEN}. "
        "Build it first: cargo build --bin sce-codegen --features cli --release -p sce-build"
    )
endif()

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

# The 3 RFC-dedicated default-marker fixtures. Lives outside the numerical
# conformance manifest because their purpose is contract testing (Default
# emission), not oracle comparison.
set(FIXTURES
    codec_default_marker_arm_a
    codec_default_marker_arm_b
    codec_variant_default_marker
)

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
        COMMENT "Generating round-trip fixture ${fixture}.h (cpp)"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS "${out_header}")
endforeach()

add_custom_target(generate_round_trip_fixtures_cpp DEPENDS ${GENERATED_HEADERS})

add_executable(default_round_trip_test default_round_trip_test.cpp)
add_dependencies(default_round_trip_test generate_round_trip_fixtures_cpp)

target_include_directories(default_round_trip_test PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_options(default_round_trip_test PRIVATE
    -Wall -Wextra -Wpedantic
)

enable_testing()
add_test(NAME default_round_trip COMMAND default_round_trip_test)
