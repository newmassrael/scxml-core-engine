# RSM Static W3C Test Code Generation
# Generates C++ state machine code from W3C SCXML test suite

# Set CMake policy CMP0116 to NEW (Ninja DEPFILE transformation)
# This suppresses warnings for add_custom_command DEPFILE usage
if(POLICY CMP0116)
    cmake_policy(SET CMP0116 NEW)
endif()

# rsm_generate_aot_test_header: Generate AOT test header (TestXXX.h) from metadata.txt
#
# Single Source of Truth: metadata.txt description is used for both Interpreter and AOT
# Eliminates description duplication between metadata.txt and TestXXX.h
#
function(rsm_generate_aot_test_header TEST_NUM TEST_TYPE)
    # Set TEST_NUMBER for template substitution (@TEST_NUMBER@ in .in files)
    set(TEST_NUMBER ${TEST_NUM})

    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(METADATA_FILE "${RESOURCE_DIR}/metadata.txt")
    set(AOT_TEST_HEADER "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/Test${TEST_NUM}.h")

    # Select template based on test type
    if("${TEST_TYPE}" STREQUAL "HTTP")
        set(TEMPLATE_FILE "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/HttpAotTestTemplate.h.in")
    else()
        set(TEMPLATE_FILE "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/SimpleAotTestTemplate.h.in")
    endif()

    # Check if metadata file exists
    if(NOT EXISTS "${METADATA_FILE}")
        message(WARNING "Metadata file not found: ${METADATA_FILE} - Skipping AOT header generation for test ${TEST_NUM}")
        return()
    endif()

    # Check if template exists
    if(NOT EXISTS "${TEMPLATE_FILE}")
        message(WARNING "Template file not found: ${TEMPLATE_FILE} - Skipping AOT header generation for test ${TEST_NUM}")
        return()
    endif()

    # Extract description and specnum from metadata.txt using Python script
    execute_process(
        COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/read_test_metadata.py" "${METADATA_FILE}"
        OUTPUT_VARIABLE TEST_DESCRIPTION
        OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE READ_METADATA_RESULT
    )

    if(NOT READ_METADATA_RESULT EQUAL 0)
        message(WARNING "Failed to read metadata for test ${TEST_NUM} - Skipping AOT header generation")
        return()
    endif()

    # Extract specnum from metadata.txt
    execute_process(
        COMMAND grep "^specnum:" "${METADATA_FILE}"
        COMMAND sed "s/specnum: *//"
        OUTPUT_VARIABLE SPECNUM
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )

    # Generate TestXXX.h from template
    configure_file(
        "${TEMPLATE_FILE}"
        "${AOT_TEST_HEADER}"
        @ONLY
    )

    message(STATUS "Generated AOT test header: Test${TEST_NUM}.h (description from metadata.txt)")
endfunction()

# rsm_generate_static_w3c_test: Generate C++ code for a single W3C test
#
# This does NOT create executable - just generates C++ header from TXML
# Automatically discovers and processes sub SCXML files (e.g., test226sub1.txml)
#
function(rsm_generate_static_w3c_test TEST_NUM OUTPUT_DIR)
    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(TXML_FILE "${RESOURCE_DIR}/test${TEST_NUM}.txml")
    set(SCXML_FILE "${OUTPUT_DIR}/test${TEST_NUM}.scxml")
    set(GENERATED_HEADER "${OUTPUT_DIR}/test${TEST_NUM}_sm.h")

    # Code generator Python scripts as base dependencies
    # Template dependencies are tracked via DEPFILE for fine-grained incremental builds
    set(CODEGEN_SCRIPTS
        "${CMAKE_SOURCE_DIR}/tools/codegen/codegen.py"
        "${CMAKE_SOURCE_DIR}/tools/codegen/scxml_parser.py"
    )

    # Check if main TXML file exists
    if(NOT EXISTS "${TXML_FILE}")
        message(WARNING "TXML file not found: ${TXML_FILE} - Skipping test ${TEST_NUM}")
        return()
    endif()

    # Auto-discover sub SCXML files (e.g., test226sub1.txml, test226sub2.txml)
    # W3C SCXML 6.2/6.4: Sub SCXML files are child state machines invoked by parent
    file(GLOB SUB_TXML_FILES "${RESOURCE_DIR}/test${TEST_NUM}sub*.txml")
    set(SUB_SCXML_DEPENDENCIES "")
    set(SUB_HEADER_DEPENDENCIES "")

    foreach(SUB_TXML_FILE ${SUB_TXML_FILES})
        get_filename_component(SUB_TXML_NAME "${SUB_TXML_FILE}" NAME_WE)
        set(SUB_SCXML_FILE "${OUTPUT_DIR}/${SUB_TXML_NAME}.scxml")
        set(SUB_HEADER_FILE "${OUTPUT_DIR}/${SUB_TXML_NAME}_sm.h")

        # Convert sub TXML to SCXML (without pass/fail validation via filename detection)
        add_custom_command(
            OUTPUT "${SUB_SCXML_FILE}"
            COMMAND txml-converter "${SUB_TXML_FILE}" "${SUB_SCXML_FILE}"
            DEPENDS txml-converter "${SUB_TXML_FILE}"
            COMMENT "Converting ${SUB_TXML_NAME}.txml to SCXML (sub state machine)"
            VERBATIM
        )

        # Generate C++ code for sub SCXML (as invoked child)
        add_custom_command(
            OUTPUT "${SUB_HEADER_FILE}"
            COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/codegen/codegen.py" "${SUB_SCXML_FILE}" -o "${OUTPUT_DIR}" --as-child --write-deps "${SUB_HEADER_FILE}.d"
            DEPENDS "${SUB_SCXML_FILE}" ${CODEGEN_SCRIPTS}
            DEPFILE "${SUB_HEADER_FILE}.d"
            COMMENT "Generating C++ code: ${SUB_TXML_NAME}_sm.h"
            VERBATIM
        )

        # Add sub SCXML to dependencies and headers
        list(APPEND SUB_SCXML_DEPENDENCIES "${SUB_SCXML_FILE}")
        list(APPEND SUB_HEADER_DEPENDENCIES "${SUB_HEADER_FILE}")
        set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} "${SUB_HEADER_FILE}" PARENT_SCOPE)
    endforeach()

    # Step 1: TXML -> SCXML conversion with name attribute
    # ARCHITECTURE.MD: CMake portability - Use ${CMAKE_COMMAND} instead of bash
    # Check if SCXML file already exists in resources (for tests with direct SCXML, e.g., test513)
    set(RESOURCE_SCXML "${RESOURCE_DIR}/test${TEST_NUM}.scxml")
    if(EXISTS "${RESOURCE_SCXML}")
        # Check if .txt file exists (only some tests have external data files)
        set(RESOURCE_TXT "${RESOURCE_DIR}/test${TEST_NUM}.txt")
        if(EXISTS "${RESOURCE_TXT}")
            # Use existing SCXML file + copy .txt file
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND ${CMAKE_COMMAND} -E copy "${RESOURCE_SCXML}" "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E copy_if_different "${RESOURCE_TXT}" "${OUTPUT_DIR}/test${TEST_NUM}.txt"
                DEPENDS "${RESOURCE_SCXML}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Using existing SCXML: test${TEST_NUM}.scxml"
                VERBATIM
            )
        else()
            # Use existing SCXML file only (no .txt file)
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND ${CMAKE_COMMAND} -E copy "${RESOURCE_SCXML}" "${SCXML_FILE}"
                DEPENDS "${RESOURCE_SCXML}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Using existing SCXML: test${TEST_NUM}.scxml"
                VERBATIM
            )
        endif()
    else()
        # Check if .txt file exists (only some tests have external data files)
        set(RESOURCE_TXT "${RESOURCE_DIR}/test${TEST_NUM}.txt")
        if(EXISTS "${RESOURCE_TXT}")
            # Convert TXML to SCXML + copy .txt file
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND txml-converter "${TXML_FILE}" "${SCXML_FILE}"
                COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/fix_scxml_name.py" "${SCXML_FILE}" "test${TEST_NUM}"
                COMMAND ${CMAKE_COMMAND} -E copy_if_different "${RESOURCE_TXT}" "${OUTPUT_DIR}/test${TEST_NUM}.txt"
                DEPENDS txml-converter "${TXML_FILE}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Converting TXML to SCXML: test${TEST_NUM}.txml"
                VERBATIM
            )
        else()
            # Convert TXML to SCXML only (no .txt file)
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND txml-converter "${TXML_FILE}" "${SCXML_FILE}"
                COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/fix_scxml_name.py" "${SCXML_FILE}" "test${TEST_NUM}"
                DEPENDS txml-converter "${TXML_FILE}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Converting TXML to SCXML: test${TEST_NUM}.txml"
                VERBATIM
            )
        endif()
    endif()

    # Step 2: SCXML -> C++ code generation (parent + inline children)
    # W3C SCXML 6.2/6.4: Parent header must depend on child headers (template detection)
    # Uses Python helper script to generate parent and process inline content children
    # ARCHITECTURE.MD: CMake portability - Use CMake scripting instead of bash
    set(CHILDREN_METADATA "${OUTPUT_DIR}/test${TEST_NUM}_children.txt")
    set(PROCESS_CHILDREN_SCRIPT "${OUTPUT_DIR}/process_children_${TEST_NUM}.cmake")

    # Generate CMake script to process inline children
    file(WRITE "${PROCESS_CHILDREN_SCRIPT}" "
        if(EXISTS \"${CHILDREN_METADATA}\")
            file(STRINGS \"${CHILDREN_METADATA}\" CHILDREN)
            foreach(child \${CHILDREN})
                if(child)
                    execute_process(
                        COMMAND python3 \"${CMAKE_SOURCE_DIR}/tools/codegen/codegen.py\"
                                \"${OUTPUT_DIR}/\${child}.scxml\" -o \"${OUTPUT_DIR}\" --as-child
                        RESULT_VARIABLE result
                    )
                    if(NOT result EQUAL 0)
                        message(WARNING \"Failed to generate child: \${child}\")
                    endif()
                endif()
            endforeach()
        endif()
    ")

    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND python3 "${CMAKE_SOURCE_DIR}/tools/codegen/codegen.py" "${SCXML_FILE}" -o "${OUTPUT_DIR}" --write-deps "${GENERATED_HEADER}.d"
        COMMAND ${CMAKE_COMMAND} -P "${PROCESS_CHILDREN_SCRIPT}"
        DEPENDS "${SCXML_FILE}" ${SUB_HEADER_DEPENDENCIES} ${CODEGEN_SCRIPTS}
        DEPFILE "${GENERATED_HEADER}.d"
        COMMENT "Generating C++ code: test${TEST_NUM}_sm.h (with inline children)"
        VERBATIM
    )

    # Add to parent scope variable
    set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} "${GENERATED_HEADER}" PARENT_SCOPE)
endfunction()

# rsm_generate_static_w3c_test_batch: Generate C++ code for multiple W3C tests
#
function(rsm_generate_static_w3c_test_batch OUTPUT_DIR)
    foreach(TEST_NUM ${ARGN})
        rsm_generate_static_w3c_test(${TEST_NUM} ${OUTPUT_DIR})
    endforeach()
endfunction()
