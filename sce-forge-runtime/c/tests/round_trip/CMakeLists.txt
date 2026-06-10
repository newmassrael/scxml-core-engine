# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity (c11 half) — runtime
# round-trip property test mirroring
# sce-forge-runtime/rust/tests/forge_default_round_trip.rs.
#
# Standalone CMake project that:
#   - regenerates sce-codegen if cargo is available
#   - generates the 3 default-marker fixtures into
#     ${CMAKE_CURRENT_BINARY_DIR}/generated/
#   - builds a small test binary that exercises the
#     <UPPER>_DEFAULT_INIT macro contract end-to-end (encode →
#     decode → byte-equal re-encode)
#
# Configure + build + test:
#   cargo build --bin sce-codegen --features cli --release -p sce-build
#   cmake -S sce-forge-runtime/c/tests/round_trip -B build_round_trip_c
#   cmake --build build_round_trip_c
#   ctest --test-dir build_round_trip_c --output-on-failure

cmake_minimum_required(VERSION 3.16)
project(sce_forge_round_trip_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/sce-forge-runtime/c/include")

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
                --language c11
                --output-dir "${GENERATED_DIR}"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        COMMENT "Generating round-trip fixture ${fixture}.h (c11)"
        VERBATIM
    )
    list(APPEND GENERATED_HEADERS "${out_header}")
endforeach()

add_custom_target(generate_round_trip_fixtures_c DEPENDS ${GENERATED_HEADERS})

add_executable(default_round_trip_test_c default_round_trip_test.c)
add_dependencies(default_round_trip_test_c generate_round_trip_fixtures_c)

target_include_directories(default_round_trip_test_c PRIVATE
    "${RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_options(default_round_trip_test_c PRIVATE
    -Wall -Wextra -Wpedantic
)

enable_testing()
add_test(NAME default_round_trip_c COMMAND default_round_trip_test_c)
