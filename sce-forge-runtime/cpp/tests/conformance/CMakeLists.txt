# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../..")
get_filename_component(REPO_ROOT "${REPO_ROOT}" ABSOLUTE)

set(SCE_CODEGEN "${REPO_ROOT}/target/release/sce-codegen")
set(RESOURCE_DIR "${REPO_ROOT}/tests/forge/resources")
set(REFERENCE_JSON "${REPO_ROOT}/tests/forge/conformance/numerical_reference.json")
set(RUNTIME_INCLUDE_DIR "${REPO_ROOT}/sce-forge-runtime/cpp/include")
set(NLOHMANN_JSON_INCLUDE_DIR "${REPO_ROOT}/third_party/nlohmann_json/include")

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

set(FIXTURES
    filter_moving_average
    filter_debounce
    interpolation_1d_linear
    interpolation_2d_bilinear
    observer_coolant
    transform_temperature
    transform_bitwise
    transform_multi_output
    condition_threshold
    condition_range
    condition_programming
    procedure_linear
    procedure_diamond
    procedure_startup_check
)

set(GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
file(MAKE_DIRECTORY "${GENERATED_DIR}")

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

add_custom_target(generate_forge_fixtures_cpp DEPENDS ${GENERATED_HEADERS})

add_executable(numerical_conformance_test
    numerical_conformance_test.cpp
)
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
