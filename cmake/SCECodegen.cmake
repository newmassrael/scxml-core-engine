# SCECodegen.cmake
# Provides sce_add_state_machine() function for automatic SCXML code generation

function(sce_add_state_machine)
    # Parse arguments: TARGET target_name SCXML_FILE file.scxml [OUTPUT_DIR dir]
    cmake_parse_arguments(SCE "" "TARGET;SCXML_FILE;OUTPUT_DIR" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machine: TARGET is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_add_state_machine: SCXML_FILE is required")
    endif()

    # Set default output directory if not specified
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Get absolute path to SCXML file
    get_filename_component(SCXML_ABS_PATH "${SCE_SCXML_FILE}" ABSOLUTE)

    # Extract base name from SCXML file
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)

    # Generated file path
    set(GENERATED_HEADER "${SCE_OUTPUT_DIR}/${SCXML_NAME}_sm.h")

    # Create output directory
    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")

    # Add custom command to generate code
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/codegen/codegen.py" "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}"
        DEPENDS "${SCXML_ABS_PATH}"
        COMMENT "Generating state machine code from ${SCXML_NAME}.scxml"
        VERBATIM
    )

    # Add generated file to target sources
    target_sources(${SCE_TARGET} PRIVATE "${GENERATED_HEADER}")

    # Add include directory for generated files
    target_include_directories(${SCE_TARGET} PRIVATE "${SCE_OUTPUT_DIR}")

    # Mark generated file as GENERATED
    set_source_files_properties("${GENERATED_HEADER}" PROPERTIES GENERATED TRUE)

    message(STATUS "SCE: Configured state machine generation for ${SCE_TARGET}")
    message(STATUS "  SCXML: ${SCXML_ABS_PATH}")
    message(STATUS "  Generated: ${GENERATED_HEADER}")
endfunction()
