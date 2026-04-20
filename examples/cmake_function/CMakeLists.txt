cmake_minimum_required(VERSION 3.14)
project(SCECMakeFunctionExample)

# C++20 standard
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# =============================================================================
# SCE CMake Function Example
# =============================================================================
# This example demonstrates using sce_add_state_machine() CMake function.
# The function is automatically available when using SCE as a subproject.
#
# Available functions:
#   sce_add_state_machine()            - Add single SCXML to target
#   sce_add_state_machines_from_dir()  - Add all SCXML files from directory
#   sce_create_state_machine_library() - Create standalone library from SCXML

# Create executable
add_executable(light_example main.cpp)

# =============================================================================
# Method 1: sce_add_state_machine() - Add to existing target
# =============================================================================
# Use SCE CMake function to automatically generate state machine code.
# This will:
#   1. Run sce-codegen to generate SimpleLight_sm.h from simple_light.scxml
#   2. Add generated file to target sources
#   3. Set up dependency tracking (regenerates when SCXML changes)
#   4. Add generated directory to include paths
sce_add_state_machine(
    TARGET light_example
    SCXML_FILE simple_light.scxml
    # OUTPUT_DIR is optional, defaults to ${CMAKE_CURRENT_BINARY_DIR}/generated
)

# Link with the pure-static tier.
# simple_light.scxml has no <cond>/<expr>/<assign location> — sce-codegen
# reports needs_script_engine=false, so the generated state machine links
# against sce_base without pulling in QuickJS or Lua.
#
# In-tree:   link sce_base directly (this file is add_subdirectory'd by SCE)
# Installed: same binary works with SCE::sce_base after find_package(SCE).
target_link_libraries(light_example PRIVATE sce_base)

# Add SCE include directories (in-tree usage)
target_include_directories(light_example PRIVATE
    ${CMAKE_SOURCE_DIR}/sce/include
)

# =============================================================================
# Method 2: sce_create_state_machine_library() - Create reusable library
# =============================================================================
# Creates a standalone INTERFACE library that can be shared between targets.
# Useful when multiple executables need the same state machine.
#
# Example usage (uncomment to try):
#
# sce_create_state_machine_library(
#     NAME simple_light_sm
#     SCXML_FILE simple_light.scxml
# )
#
# add_executable(another_app another_main.cpp)
# target_link_libraries(another_app PRIVATE simple_light_sm sce_runtime)

# =============================================================================
# Method 3: sce_add_state_machines_from_dir() - Batch generation
# =============================================================================
# Generate state machines from all SCXML files in a directory.
# Useful for projects with many state machines.
#
# Example usage (uncomment to try):
#
# sce_add_state_machines_from_dir(
#     TARGET light_example
#     SCXML_DIR ${CMAKE_CURRENT_SOURCE_DIR}/scxml
# )
