# SCECodegen.cmake
# Provides sce_add_state_machine() function for automatic SCXML code generation
#
# This file works both:
#   1. In-tree: When SCE is included as subdirectory (add_subdirectory)
#   2. Installed: When SCE is found via find_package(SCE)
#
# Usage:
#   sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
#   sce_add_state_machines_from_dir(TARGET my_app SCXML_DIR scxml/)

# Determine codegen script location
# Priority: 1) SCE_CODEGEN_SCRIPT (set by installed package)
#           2) In-tree development path
if(NOT DEFINED SCE_CODEGEN_SCRIPT)
    # In-tree development: script is relative to this cmake file
    get_filename_component(_SCE_CMAKE_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)
    set(SCE_CODEGEN_SCRIPT "${_SCE_CMAKE_ROOT}/tools/codegen/codegen.py")

    if(NOT EXISTS "${SCE_CODEGEN_SCRIPT}")
        message(FATAL_ERROR "SCE: codegen.py not found at ${SCE_CODEGEN_SCRIPT}")
    endif()
endif()

# Find Python3 if not already found
if(NOT Python3_EXECUTABLE)
    find_package(Python3 REQUIRED COMPONENTS Interpreter)
endif()

#[=============================================================================[
sce_add_state_machine(TARGET target SCXML_FILE file.scxml [OUTPUT_DIR dir])

Generates C++ state machine header from SCXML and adds to target.

Arguments:
  TARGET      - CMake target to add generated code to (required)
  SCXML_FILE  - Path to SCXML file (required)
  OUTPUT_DIR  - Output directory for generated files (optional, defaults to
                ${CMAKE_CURRENT_BINARY_DIR}/generated)

Example:
  add_executable(my_app main.cpp)
  sce_add_state_machine(TARGET my_app SCXML_FILE player.scxml)
  target_link_libraries(my_app PRIVATE SCE::sce)
#]=============================================================================]
function(sce_add_state_machine)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_FILE;OUTPUT_DIR" "" ${ARGN})

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
        message(FATAL_ERROR "sce_add_state_machine: SCXML file not found: ${SCXML_ABS_PATH}")
    endif()

    # Create output directory
    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")

    # Collect Jinja2 template dependencies for incremental rebuild
    get_filename_component(_SCE_CODEGEN_DIR "${SCE_CODEGEN_SCRIPT}" DIRECTORY)
    file(GLOB _SCE_TEMPLATES "${_SCE_CODEGEN_DIR}/templates/*.jinja2" "${_SCE_CODEGEN_DIR}/templates/actions/*.jinja2")

    # Add custom command to generate state machine header
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND "${Python3_EXECUTABLE}" "${SCE_CODEGEN_SCRIPT}"
                "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        DEPENDS "${SCXML_ABS_PATH}" "${SCE_CODEGEN_SCRIPT}" ${_SCE_TEMPLATES}
        COMMENT "SCE: Generating ${SCXML_NAME}_sm.h from SCXML"
        VERBATIM
    )

    # Add generated file to target
    target_sources(${SCE_TARGET} PRIVATE "${GENERATED_HEADER}")
    target_include_directories(${SCE_TARGET} PRIVATE "${SCE_OUTPUT_DIR}")
    set_source_files_properties("${GENERATED_HEADER}" PROPERTIES GENERATED TRUE)

    message(STATUS "SCE: Added state machine '${SCXML_NAME}' to target '${SCE_TARGET}'")
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
    cmake_parse_arguments(SCE "" "TARGET;SCXML_DIR;OUTPUT_DIR" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: TARGET is required")
    endif()

    if(NOT SCE_SCXML_DIR)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: SCXML_DIR is required")
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
  target_link_libraries(my_app PRIVATE player_sm SCE::sce)
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

    # Collect Jinja2 template dependencies for incremental rebuild
    get_filename_component(_SCE_CODEGEN_DIR "${SCE_CODEGEN_SCRIPT}" DIRECTORY)
    file(GLOB _SCE_TEMPLATES "${_SCE_CODEGEN_DIR}/templates/*.jinja2" "${_SCE_CODEGEN_DIR}/templates/actions/*.jinja2")

    # Add custom command to generate state machine header
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND "${Python3_EXECUTABLE}" "${SCE_CODEGEN_SCRIPT}"
                "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        DEPENDS "${SCXML_ABS_PATH}" "${SCE_CODEGEN_SCRIPT}" ${_SCE_TEMPLATES}
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
