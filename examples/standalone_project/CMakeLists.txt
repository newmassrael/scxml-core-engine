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
# find_package(SCE) provides four tier targets (ARCHITECTURE.md §4-Tier):
#   - SCE::sce_core       — header-only (minimal embed)
#   - SCE::sce_base       — pure static AOT (no scripting)
#   - SCE::sce_scripting  — static hybrid AOT with JSEngine  ← this example
#   - SCE::sce_runtime    — full interpreter + parser + HTTP
# Plus CMake functions:
#   - sce_add_state_machine(), sce_add_state_machines_from_dir(),
#     sce_create_state_machine_library()

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
# Link with the Scripting Tier
# =============================================================================
# traffic_light.scxml uses <log expr="..."/> — sce-codegen reports
# needs_script_engine=true, so the generated header expects JSEngine to
# evaluate expressions at runtime. Link SCE::sce_scripting (static hybrid AOT).
#
# If this example were pure static (no <cond>/<expr>/<assign location=…>),
# SCE::sce_base would suffice and the compiled binary would have zero
# dependency on QuickJS or Lua.

target_link_libraries(traffic_light PRIVATE SCE::sce_scripting)
