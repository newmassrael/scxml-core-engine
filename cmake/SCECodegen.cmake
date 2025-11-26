# SCECodegen.cmake
# Provides sce_add_state_machine() function for automatic SCXML code generation

# Determine SCE root directory (this file is in ${SCE_ROOT}/cmake/)
get_filename_component(SCE_CMAKE_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)
set(SCE_CODEGEN_SCRIPT "${SCE_CMAKE_ROOT}/tools/codegen/codegen.py")

# sce_add_state_machine(TARGET target SCXML_FILE file.scxml [OUTPUT_DIR dir])
# Generates C++ state machine header from SCXML and adds to target
function(sce_add_state_machine)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_FILE;OUTPUT_DIR" "" ${ARGN})

    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machine: TARGET is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_add_state_machine: SCXML_FILE is required")
    endif()

    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    get_filename_component(SCXML_ABS_PATH "${SCE_SCXML_FILE}" ABSOLUTE)
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)
    set(GENERATED_HEADER "${SCE_OUTPUT_DIR}/${SCXML_NAME}_sm.h")

    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")

    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND python3 "${SCE_CODEGEN_SCRIPT}" "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        DEPENDS "${SCXML_ABS_PATH}" "${SCE_CODEGEN_SCRIPT}"
        COMMENT "Generating ${SCXML_NAME}_sm.h from SCXML"
        VERBATIM
    )

    target_sources(${SCE_TARGET} PRIVATE "${GENERATED_HEADER}")
    target_include_directories(${SCE_TARGET} PRIVATE "${SCE_OUTPUT_DIR}")
    set_source_files_properties("${GENERATED_HEADER}" PROPERTIES GENERATED TRUE)
endfunction()

# sce_add_state_machines_from_dir(TARGET target SCXML_DIR dir [OUTPUT_DIR dir])
# Finds all *.scxml files in directory and generates state machines
function(sce_add_state_machines_from_dir)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_DIR;OUTPUT_DIR" "" ${ARGN})

    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: TARGET is required")
    endif()

    if(NOT SCE_SCXML_DIR)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: SCXML_DIR is required")
    endif()

    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Find all SCXML files
    file(GLOB SCXML_FILES "${SCE_SCXML_DIR}/*.scxml")

    if(NOT SCXML_FILES)
        message(WARNING "No SCXML files found in ${SCE_SCXML_DIR}")
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
