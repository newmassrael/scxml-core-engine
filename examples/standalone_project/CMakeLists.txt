cmake_minimum_required(VERSION 3.14)
project(SCEStandaloneExample VERSION 1.0.0)

# =============================================================================
# Standalone SCE Project Example
# =============================================================================
# This example demonstrates how to use SCE (SCXML Core Engine) in a standalone
# project using find_package(SCE). This is the recommended approach for
# production projects.
#
# Prerequisites:
#   1. Build and install SCE:
#      cd /path/to/scxml-core-engine/build
#      cmake .. -DCMAKE_INSTALL_PREFIX=/usr/local
#      make && sudo make install
#
#   2. Build this project:
#      mkdir build && cd build
#      cmake ..
#      make
#
# Alternative: Use SCE from build directory without installing:
#      cmake .. -DCMAKE_PREFIX_PATH=/path/to/scxml-core-engine/build
#
# Or specify SCE_ROOT for explicit path:
#      cmake .. -DSCE_ROOT=/path/to/scxml-core-engine

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# =============================================================================
# Find SCE Package
# =============================================================================
# find_package(SCE) provides:
#   - SCE::sce target (main library)
#   - sce_add_state_machine() function
#   - sce_add_state_machines_from_dir() function
#   - sce_create_state_machine_library() function

find_package(SCE REQUIRED)

# =============================================================================
# Create Executable
# =============================================================================

add_executable(traffic_light main.cpp)

# =============================================================================
# Generate State Machine from SCXML
# =============================================================================
# sce_add_state_machine() automatically:
#   1. Runs sce-codegen to generate C++ header
#   2. Adds generated file to target
#   3. Sets up include directories
#   4. Tracks dependencies (regenerates on SCXML change)

sce_add_state_machine(
    TARGET traffic_light
    SCXML_FILE traffic_light.scxml
)

# =============================================================================
# Link with SCE Library
# =============================================================================
# SCE::sce is the main library target providing:
#   - StaticExecutionEngine for AOT state machines
#   - InterpreterEngine for dynamic SCXML loading
#   - Event dispatcher interfaces

target_link_libraries(traffic_light PRIVATE SCE::sce)
