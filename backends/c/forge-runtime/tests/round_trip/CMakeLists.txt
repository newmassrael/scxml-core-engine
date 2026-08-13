# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity (c11 half) — runtime
# round-trip property test mirroring
# backends/rust/forge-runtime/tests/forge_default_round_trip.rs.
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
#   cargo build --bin sce-codegen --features cli -p sce-build
#   cmake -S backends/c/forge-runtime/tests/round_trip -B build_round_trip_c
#   cmake --build build_round_trip_c
#   ctest --test-dir build_round_trip_c --output-on-failure

cmake_minimum_required(VERSION 3.16)
project(sce_forge_round_trip_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/forge-runtime/include")
# Tier 1 header tree. Generated forge C headers reach into it for
# `<sce/sample.h>` (ownership contract) and `<sce/portability.h>`
# (`SCE_STATIC_ASSERT`); see backends/c/forge-runtime/CMakeLists.txt.
set(CORE_RUNTIME_INCLUDE_DIR "${REPO_ROOT}/backends/c/runtime/include")

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

# Resolve the generator. The search lives in cmake/SCEFindCodegen.cmake
# so every harness finds it the same way and none of them names a
# build profile; it aborts with the build command when no profile
# holds a binary, which is what the hand-written check here used to do.
include("${REPO_ROOT}/cmake/SCEFindCodegen.cmake")

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
        COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${scxml}"
                --language c11
                --output-dir "${GENERATED_DIR}"
                --write-deps "${out_header}.d"
        DEPENDS "${scxml}" "${SCE_CODEGEN}"
        DEPFILE "${out_header}.d"
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
    "${CORE_RUNTIME_INCLUDE_DIR}"
    "${GENERATED_DIR}"
)

target_compile_options(default_round_trip_test_c PRIVATE
    -Wall -Wextra -Wpedantic
)

enable_testing()
add_test(NAME default_round_trip_c COMMAND default_round_trip_test_c)
