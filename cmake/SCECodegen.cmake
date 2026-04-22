# SCECodegen.cmake
# Provides sce_add_state_machine() function for automatic SCXML code generation
#
# This file works both:
#   1. In-tree: When SCE is included as subdirectory (add_subdirectory)
#   2. Installed: When SCE is found via find_package(SCE)
#
# Uses sce-codegen (Rust binary) for code generation.
# Build: cargo build --bin sce-codegen --features cli --release -p sce-build
#
# Usage:
#   sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
#   sce_add_state_machines_from_dir(TARGET my_app SCXML_DIR scxml/)

# Find sce-codegen binary
# Priority: 1) SCE_CODEGEN (already set by parent CMake / installed package)
#           2) In-tree cargo build output (release, then debug)
#           3) System PATH
if(NOT SCE_CODEGEN)
    get_filename_component(_SCE_CMAKE_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)
    find_program(SCE_CODEGEN sce-codegen
        PATHS "${_SCE_CMAKE_ROOT}/target/release" "${_SCE_CMAKE_ROOT}/target/debug"
        NO_DEFAULT_PATH
    )
    if(NOT SCE_CODEGEN)
        find_program(SCE_CODEGEN sce-codegen)
    endif()
endif()

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR "SCE: sce-codegen not found. Build it with: cargo build --bin sce-codegen --features cli --release -p sce-build")
endif()

message(STATUS "SCE: Using code generator: ${SCE_CODEGEN}")

# Auto-detect SCE_TEMPLATE_DIR so release sce-codegen binaries (which
# refuse silent fallback — see sce-build::find_template_base) find the
# Jinja2 templates in all three distribution layouts without the
# consumer having to set SCE_TEMPLATE_DIR manually:
#
#   In-tree:        cmake/      <-> ../tools/codegen/templates
#   Embed vendor:   embed/      <-> tools/codegen/templates (alongside)
#   System install: lib/cmake/SCE/ <-> ../../share/sce/codegen/templates
#
# Consumer-set SCE_TEMPLATE_DIR wins; auto-detection only runs when unset.
if(NOT SCE_TEMPLATE_DIR)
    if(EXISTS "${CMAKE_CURRENT_LIST_DIR}/../tools/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/../tools/codegen/templates")
    elseif(EXISTS "${CMAKE_CURRENT_LIST_DIR}/tools/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/tools/codegen/templates")
    elseif(EXISTS "${CMAKE_CURRENT_LIST_DIR}/../../share/sce/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/../../share/sce/codegen/templates")
    endif()
    if(SCE_TEMPLATE_DIR)
        message(STATUS "SCE: Auto-detected template dir: ${SCE_TEMPLATE_DIR}")
    endif()
endif()

# clang-format integration for generated C++ code
include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

#[=============================================================================[
sce_add_state_machine(TARGET target SCXML_FILE file.scxml [OUTPUT_DIR dir] [LANGUAGE lang])

Generates state machine code from SCXML and adds to target.

Arguments:
  TARGET      - CMake target to add generated code to (required)
  SCXML_FILE  - Path to SCXML file (required)
  OUTPUT_DIR  - Output directory for generated files (optional, defaults to
                ${CMAKE_CURRENT_BINARY_DIR}/generated)
  LANGUAGE    - Target language: cpp (default), rust, kotlin, or go

Example:
  add_executable(my_app main.cpp)
  sce_add_state_machine(TARGET my_app SCXML_FILE player.scxml)
  # Link the tier matching the SCXML feature set (ARCHITECTURE.md §4-Tier):
  #   SCE::sce_base      — pure static (no <cond>/<expr>/<assign location>)
  #   SCE::sce_scripting — static hybrid (needs_script_engine=true)
  #   SCE::sce_runtime   — full interpreter (dynamic SCXML loading)
  target_link_libraries(my_app PRIVATE SCE::sce_base)
#]=============================================================================]
function(sce_add_state_machine)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_FILE;OUTPUT_DIR;LANGUAGE" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machine: TARGET is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_add_state_machine: SCXML_FILE is required")
    endif()

    # Verify target exists
    if(NOT TARGET ${SCE_TARGET})
        message(FATAL_ERROR "sce_add_state_machine: TARGET '${SCE_TARGET}' does not exist")
    endif()

    # Set default language
    if(NOT SCE_LANGUAGE)
        set(SCE_LANGUAGE "cpp")
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Get absolute path and extract name
    get_filename_component(SCXML_ABS_PATH "${SCE_SCXML_FILE}" ABSOLUTE)
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)

    # Language-specific output file naming
    if(SCE_LANGUAGE STREQUAL "kotlin")
        set(GENERATED_OUTPUT "${SCE_OUTPUT_DIR}/${SCXML_NAME}Sm.kt")
    else()
        set(GENERATED_OUTPUT "${SCE_OUTPUT_DIR}/${SCXML_NAME}_sm.h")
    endif()

    # Verify SCXML file exists
    if(NOT EXISTS "${SCXML_ABS_PATH}")
        message(FATAL_ERROR "sce_add_state_machine: SCXML file not found: ${SCXML_ABS_PATH}")
    endif()

    # Create output directory
    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")

    # DEPFILE path for incremental build tracking (must be set before use in command)
    set(_SCE_DEPFILE "${GENERATED_OUTPUT}.d")

    # Build codegen command with optional template dir (installed package scenario)
    set(_SCE_CODEGEN_CMD "${SCE_CODEGEN}" generate
        "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        -l "${SCE_LANGUAGE}"
        --write-deps "${_SCE_DEPFILE}")
    if(SCE_TEMPLATE_DIR)
        set(_SCE_CODEGEN_CMD ${CMAKE_COMMAND} -E env "SCE_TEMPLATE_DIR=${SCE_TEMPLATE_DIR}" ${_SCE_CODEGEN_CMD})
    endif()

    # clang-format post-processing for C++ output (no-op if not available or non-C++)
    if(SCE_CLANG_FORMAT_FOUND AND SCE_LANGUAGE STREQUAL "cpp")
        set(_SCE_INL_OUTPUT "${SCE_OUTPUT_DIR}/${SCXML_NAME}_sm.inl")
        set(_SCE_FORMAT_CMD COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}" -i "${GENERATED_OUTPUT}" "${_SCE_INL_OUTPUT}")
    else()
        set(_SCE_FORMAT_CMD "")
    endif()

    # Add custom command to generate state machine code
    # Uses DEPFILE for fine-grained template dependency tracking
    add_custom_command(
        OUTPUT "${GENERATED_OUTPUT}"
        COMMAND ${_SCE_CODEGEN_CMD}
        ${_SCE_FORMAT_CMD}
        DEPENDS "${SCXML_ABS_PATH}" "${SCE_CODEGEN}"
        DEPFILE "${_SCE_DEPFILE}"
        COMMENT "SCE: Generating ${SCXML_NAME} (${SCE_LANGUAGE}) from SCXML"
        VERBATIM
    )

    # Language-specific target integration
    if(SCE_LANGUAGE STREQUAL "kotlin")
        # Kotlin: .kt files are not compiled by CMake — Gradle handles Kotlin compilation.
        # Create a custom target so CMake tracks the generation dependency.
        add_custom_target(${SCE_TARGET}_${SCXML_NAME}_kt DEPENDS "${GENERATED_OUTPUT}")
        add_dependencies(${SCE_TARGET} ${SCE_TARGET}_${SCXML_NAME}_kt)
    else()
        # C++: Add generated header to target sources and include path
        target_sources(${SCE_TARGET} PRIVATE "${GENERATED_OUTPUT}")
        target_include_directories(${SCE_TARGET} PRIVATE "${SCE_OUTPUT_DIR}")
        set_source_files_properties("${GENERATED_OUTPUT}" PROPERTIES GENERATED TRUE)
    endif()

    message(STATUS "SCE: Added state machine '${SCXML_NAME}' (${SCE_LANGUAGE}) to target '${SCE_TARGET}'")
endfunction()

#[=============================================================================[
sce_add_state_machines_from_dir(TARGET target SCXML_DIR dir [OUTPUT_DIR dir])

Finds all *.scxml files in directory and generates state machines.

Arguments:
  TARGET      - CMake target to add generated code to (required)
  SCXML_DIR   - Directory containing SCXML files (required)
  OUTPUT_DIR  - Output directory for generated files (optional)

NOTE: This function uses file(GLOB) which only runs at configure time.
      If you add new SCXML files to the directory, you must reconfigure
      (run cmake again) for them to be detected. For explicit control,
      use sce_add_state_machine() for each file instead.

Example:
  add_executable(my_app main.cpp)
  sce_add_state_machines_from_dir(TARGET my_app SCXML_DIR ${CMAKE_SOURCE_DIR}/scxml)
#]=============================================================================]
function(sce_add_state_machines_from_dir)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_DIR;OUTPUT_DIR;LANGUAGE" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: TARGET is required")
    endif()

    if(NOT SCE_SCXML_DIR)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: SCXML_DIR is required")
    endif()

    # Set default language
    if(NOT SCE_LANGUAGE)
        set(SCE_LANGUAGE "cpp")
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Find all SCXML files
    file(GLOB SCXML_FILES "${SCE_SCXML_DIR}/*.scxml")

    if(NOT SCXML_FILES)
        message(WARNING "SCE: No SCXML files found in ${SCE_SCXML_DIR}")
        return()
    endif()

    # Generate state machine for each SCXML file
    foreach(SCXML_FILE ${SCXML_FILES})
        sce_add_state_machine(
            TARGET ${SCE_TARGET}
            SCXML_FILE ${SCXML_FILE}
            OUTPUT_DIR ${SCE_OUTPUT_DIR}
            LANGUAGE ${SCE_LANGUAGE}
        )
    endforeach()

    list(LENGTH SCXML_FILES SCXML_COUNT)
    message(STATUS "SCE: Added ${SCXML_COUNT} state machines from ${SCE_SCXML_DIR}")
endfunction()

#[=============================================================================[
sce_create_state_machine_library(NAME name SCXML_FILE file.scxml [OUTPUT_DIR dir])

Creates a standalone INTERFACE library from SCXML file.
Useful when you want to share generated code between multiple targets.

Arguments:
  NAME        - Name for the generated library (required)
  SCXML_FILE  - Path to SCXML file (required)
  OUTPUT_DIR  - Output directory for generated files (optional)

Example:
  sce_create_state_machine_library(NAME player_sm SCXML_FILE player.scxml)
  # Pick the SCE tier matching the SCXML feature set — see ARCHITECTURE.md §4-Tier.
  target_link_libraries(my_app PRIVATE player_sm SCE::sce_base)
#]=============================================================================]
function(sce_create_state_machine_library)
    cmake_parse_arguments(SCE "" "NAME;SCXML_FILE;OUTPUT_DIR" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_NAME)
        message(FATAL_ERROR "sce_create_state_machine_library: NAME is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_create_state_machine_library: SCXML_FILE is required")
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Get absolute path and extract name
    get_filename_component(SCXML_ABS_PATH "${SCE_SCXML_FILE}" ABSOLUTE)
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)
    set(GENERATED_HEADER "${SCE_OUTPUT_DIR}/${SCXML_NAME}_sm.h")

    # Verify SCXML file exists
    if(NOT EXISTS "${SCXML_ABS_PATH}")
        message(FATAL_ERROR "sce_create_state_machine_library: SCXML file not found: ${SCXML_ABS_PATH}")
    endif()

    # Create output directory
    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")

    # Build codegen command with optional template dir (installed package scenario)
    set(_SCE_DEPFILE "${GENERATED_HEADER}.d")
    set(_SCE_CODEGEN_CMD "${SCE_CODEGEN}" generate
        "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        --write-deps "${_SCE_DEPFILE}")
    if(SCE_TEMPLATE_DIR)
        set(_SCE_CODEGEN_CMD ${CMAKE_COMMAND} -E env "SCE_TEMPLATE_DIR=${SCE_TEMPLATE_DIR}" ${_SCE_CODEGEN_CMD})
    endif()

    # Add custom command to generate state machine header
    # Uses DEPFILE for fine-grained template dependency tracking
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND ${_SCE_CODEGEN_CMD}
        DEPENDS "${SCXML_ABS_PATH}" "${SCE_CODEGEN}"
        DEPFILE "${_SCE_DEPFILE}"
        COMMENT "SCE: Generating ${SCXML_NAME}_sm.h library"
        VERBATIM
    )

    # Create custom target to ensure generation happens
    add_custom_target(${SCE_NAME}_gen DEPENDS "${GENERATED_HEADER}")

    # Create INTERFACE library
    add_library(${SCE_NAME} INTERFACE)
    add_dependencies(${SCE_NAME} ${SCE_NAME}_gen)
    target_include_directories(${SCE_NAME} INTERFACE "${SCE_OUTPUT_DIR}")

    message(STATUS "SCE: Created state machine library '${SCE_NAME}' from ${SCXML_NAME}.scxml")
endfunction()
