# SCE Static W3C Test Code Generation
# Generates C++ state machine code from W3C SCXML test suite
#
# Requires sce-codegen (Rust binary).
# Build: cargo build --bin sce-codegen --features cli --release -p sce-build

# Set CMake policy CMP0116 to NEW (Ninja DEPFILE transformation)
# This suppresses warnings for add_custom_command DEPFILE usage
if(POLICY CMP0116)
    cmake_policy(SET CMP0116 NEW)
endif()

# clang-format integration for generated C++ code
include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

# Find sce-codegen binary (built via: cargo build --bin sce-codegen --features cli --release -p sce-build)
find_program(SCE_CODEGEN sce-codegen
    PATHS "${CMAKE_SOURCE_DIR}/target/release" "${CMAKE_SOURCE_DIR}/target/debug"
    NO_DEFAULT_PATH
)
if(NOT SCE_CODEGEN)
    # Fallback: check system PATH
    find_program(SCE_CODEGEN sce-codegen)
endif()

if(SCE_CODEGEN)
    message(STATUS "SCE: Using code generator: ${SCE_CODEGEN}")
else()
    message(FATAL_ERROR "SCE: sce-codegen not found. Build it with: cargo build --bin sce-codegen --features cli --release -p sce-build")
endif()

# sce_generate_aot_test_header: Generate AOT test header (TestXXX.h) from metadata.txt
#
# Single Source of Truth: metadata.txt description is used for both Interpreter and AOT
# Eliminates description duplication between metadata.txt and TestXXX.h
#
function(sce_generate_aot_test_header TEST_NUM TEST_TYPE)
    # Set TEST_NUMBER for template substitution (@TEST_NUMBER@ in .in files)
    set(TEST_NUMBER ${TEST_NUM})

    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(METADATA_FILE "${RESOURCE_DIR}/metadata.txt")
    set(AOT_TEST_HEADER "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/Test${TEST_NUM}.h")

    # Skip generation if header already exists (preserve hand-crafted files
    # with custom PASS_STATE, timeouts, or detailed documentation)
    if(EXISTS "${AOT_TEST_HEADER}")
        return()
    endif()

    # Select template based on test type
    if("${TEST_TYPE}" STREQUAL "HTTP")
        set(TEMPLATE_FILE "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/HttpAotTestTemplate.h.in")
    elseif("${TEST_TYPE}" STREQUAL "SCHEDULED")
        set(TEMPLATE_FILE "${CMAKE_SOURCE_DIR}/tests/w3c/aot_tests/ScheduledAotTestTemplate.h.in")
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

    # Extract description from metadata.txt
    execute_process(
        COMMAND "${SCE_CODEGEN}" read-metadata "${METADATA_FILE}"
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

# sce_generate_static_w3c_test: Generate C++ code for a single W3C test
#
# This does NOT create executable - just generates C++ header from TXML
# Automatically discovers and processes sub SCXML files (e.g., test226sub1.txml)
#
function(sce_generate_static_w3c_test TEST_NUM OUTPUT_DIR)
    # Parse optional TYPE parameter (SIMPLE, SCHEDULED, HTTP)
    cmake_parse_arguments(_SWT "" "TYPE" "" ${ARGN})
    if(NOT _SWT_TYPE)
        set(_SWT_TYPE "SIMPLE")
    endif()

    # W3C Test Registration Automation:
    # 1. Accumulate test number into W3C_AOT_TESTS (eliminates manual list duplication)
    list(APPEND W3C_AOT_TESTS ${TEST_NUM})
    set(W3C_AOT_TESTS ${W3C_AOT_TESTS} PARENT_SCOPE)

    # 2. Auto-generate TestXXX.h from template if not exists
    sce_generate_aot_test_header(${TEST_NUM} "${_SWT_TYPE}")

    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(TXML_FILE "${RESOURCE_DIR}/test${TEST_NUM}.txml")
    set(SCXML_FILE "${OUTPUT_DIR}/test${TEST_NUM}.scxml")
    set(GENERATED_HEADER "${OUTPUT_DIR}/test${TEST_NUM}_sm.h")
    set(GENERATED_INL "${OUTPUT_DIR}/test${TEST_NUM}_sm.inl")

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
        set(SUB_INL_FILE "${OUTPUT_DIR}/${SUB_TXML_NAME}_sm.inl")
        # clang-format post-processing (no-op if not available)
        if(SCE_CLANG_FORMAT_FOUND)
            set(_SUB_FORMAT_CMD COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}" -i "${SUB_HEADER_FILE}" "${SUB_INL_FILE}")
        else()
            set(_SUB_FORMAT_CMD "")
        endif()
        add_custom_command(
            OUTPUT "${SUB_HEADER_FILE}"
            COMMAND "${SCE_CODEGEN}" generate "${SUB_SCXML_FILE}" -l cpp -o "${OUTPUT_DIR}" --as-child --write-deps "${SUB_HEADER_FILE}.d"
            ${_SUB_FORMAT_CMD}
            DEPENDS "${SUB_SCXML_FILE}" "${SCE_CODEGEN}"
            DEPFILE "${SUB_HEADER_FILE}.d"
            BYPRODUCTS "${SUB_INL_FILE}"
            COMMENT "Generating C++ code: ${SUB_TXML_NAME}_sm.h + .inl"
            VERBATIM
        )

        # Add sub SCXML to dependencies and headers
        list(APPEND SUB_SCXML_DEPENDENCIES "${SUB_SCXML_FILE}")
        list(APPEND SUB_HEADER_DEPENDENCIES "${SUB_HEADER_FILE}")
        # Update both local scope (for line 228) and parent scope (for caller)
        list(APPEND GENERATED_W3C_HEADERS "${SUB_HEADER_FILE}")
        set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} PARENT_SCOPE)
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
        # TXML -> SCXML conversion + name attribute fix
        set(RESOURCE_TXT "${RESOURCE_DIR}/test${TEST_NUM}.txt")
        if(EXISTS "${RESOURCE_TXT}")
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND txml-converter "${TXML_FILE}" "${SCXML_FILE}"
                COMMAND "${SCE_CODEGEN}" fix-scxml-name "${SCXML_FILE}" "test${TEST_NUM}"
                COMMAND ${CMAKE_COMMAND} -E copy_if_different "${RESOURCE_TXT}" "${OUTPUT_DIR}/test${TEST_NUM}.txt"
                DEPENDS txml-converter "${TXML_FILE}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Converting TXML to SCXML: test${TEST_NUM}.txml"
                VERBATIM
            )
        else()
            add_custom_command(
                OUTPUT "${SCXML_FILE}"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
                COMMAND txml-converter "${TXML_FILE}" "${SCXML_FILE}"
                COMMAND "${SCE_CODEGEN}" fix-scxml-name "${SCXML_FILE}" "test${TEST_NUM}"
                DEPENDS txml-converter "${TXML_FILE}" ${SUB_SCXML_DEPENDENCIES}
                COMMENT "Converting TXML to SCXML: test${TEST_NUM}.txml"
                VERBATIM
            )
        endif()
    endif()

    # Step 2: SCXML -> C++ code generation (parent + inline children)
    # W3C SCXML 6.2/6.4: Parent header must depend on child headers (template detection)
    set(CHILDREN_METADATA "${OUTPUT_DIR}/test${TEST_NUM}_children.txt")
    set(PROCESS_CHILDREN_SCRIPT "${OUTPUT_DIR}/process_children_${TEST_NUM}.cmake")

    # Generate CMake script to process inline children
    file(WRITE "${PROCESS_CHILDREN_SCRIPT}" "
        if(EXISTS \"${CHILDREN_METADATA}\")
            file(STRINGS \"${CHILDREN_METADATA}\" CHILDREN)
            foreach(child \${CHILDREN})
                if(child)
                    execute_process(
                        COMMAND \"${SCE_CODEGEN}\" generate
                                \"${OUTPUT_DIR}/\${child}.scxml\" -l cpp -o \"${OUTPUT_DIR}\" --as-child
                        RESULT_VARIABLE result
                    )
                    if(NOT result EQUAL 0)
                        message(WARNING \"Failed to generate child: \${child}\")
                    endif()
                endif()
            endforeach()
        endif()
    ")

    # clang-format post-processing (no-op if not available)
    if(SCE_CLANG_FORMAT_FOUND)
        set(_PARENT_FORMAT_CMD COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}" -i "${GENERATED_HEADER}" "${GENERATED_INL}")
    else()
        set(_PARENT_FORMAT_CMD "")
    endif()
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND "${SCE_CODEGEN}" generate "${SCXML_FILE}" -l cpp -o "${OUTPUT_DIR}" --write-deps "${GENERATED_HEADER}.d"
        COMMAND ${CMAKE_COMMAND} -P "${PROCESS_CHILDREN_SCRIPT}"
        ${_PARENT_FORMAT_CMD}
        DEPENDS "${SCXML_FILE}" ${SUB_HEADER_DEPENDENCIES} "${SCE_CODEGEN}"
        DEPFILE "${GENERATED_HEADER}.d"
        BYPRODUCTS "${GENERATED_INL}"
        COMMENT "Generating C++ code: test${TEST_NUM}_sm.h + .inl"
        VERBATIM
    )

    # Add to parent scope variable (update both local and parent scopes)
    list(APPEND GENERATED_W3C_HEADERS "${GENERATED_HEADER}")
    set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} PARENT_SCOPE)
endfunction()

# sce_generate_static_w3c_test_batch: Generate C++ code for multiple W3C tests
#
# NOTE: PARENT_SCOPE propagation for W3C_AOT_TESTS and GENERATED_W3C_HEADERS
# requires explicit forwarding from this wrapper function to the caller's scope.
function(sce_generate_static_w3c_test_batch OUTPUT_DIR)
    foreach(TEST_NUM ${ARGN})
        sce_generate_static_w3c_test(${TEST_NUM} ${OUTPUT_DIR})
    endforeach()
    # Propagate accumulated variables to caller's scope (grandparent of inner calls)
    set(W3C_AOT_TESTS ${W3C_AOT_TESTS} PARENT_SCOPE)
    set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} PARENT_SCOPE)
endfunction()

# sce_generate_static_w3c_c_test: Generate C11 code for a single W3C test
#
# RFC §5.J.1 sibling of `sce_generate_static_w3c_test`. Produces a `.h` + `.c`
# pair under OUTPUT_DIR via `sce-codegen -l c11`. Sub-state-machine and
# child-invoke discovery are deferred — the C11 emitter ships the
# datamodel-less + ECMAScript-via-Lua subset, which has no `<invoke>`
# surface to recurse into. When invoke support lands the function will
# mirror the C++ version's sub-TXML loop.
#
# Optional flag NEEDS_LUA: append the test to W3C_C_AOT_TESTS_NEEDS_LUA so
# the caller can conditionally `target_link_libraries(... lua54)` only for
# fixtures whose codegen embedded `luaL_dostring` calls. Datamodel-less
# fixtures (e.g. test355, test144) leave Lua out of the link entirely so
# the MCU footprint stays minimal for the watching-zenoh consumer.
#
# Optional flag NEEDS_DOM: append the test to W3C_C_AOT_TESTS_NEEDS_DOM so
# the caller can link the host-side XML DOM helper (sce-c-tests/support/
# dom.c + lua_dom_binding.c) alongside Lua. NEEDS_DOM implies NEEDS_LUA.
# Currently used only by the W3C SCXML B.2 corpus (test557, test561) —
# the helper is testbench-only and never enters the sce-c-runtime link.
#
# Optional flag NEEDS_HTTP: append the test to W3C_C_AOT_TESTS_NEEDS_HTTP so
# the caller can link the host-side HTTP/1.1 client + JSON extractor
# (sce-c-tests/support/http_client.c). Used by the W3C SCXML C.2
# BasicHTTPEventProcessor corpus that issues a real HTTP POST against the
# Node.js standalone server (`tests/w3c/standalone_http_server.js`).
# Independent of NEEDS_LUA / NEEDS_DOM — combined freely as the fixture
# requires.
#
function(sce_generate_static_w3c_c_test TEST_NUM OUTPUT_DIR)
    cmake_parse_arguments(_SWCT "NEEDS_LUA;NEEDS_DOM;NEEDS_HTTP" "" "" ${ARGN})

    # Accumulate test number into W3C_C_AOT_TESTS so the caller can iterate.
    list(APPEND W3C_C_AOT_TESTS ${TEST_NUM})
    set(W3C_C_AOT_TESTS ${W3C_C_AOT_TESTS} PARENT_SCOPE)
    if(_SWCT_NEEDS_LUA OR _SWCT_NEEDS_DOM)
        list(APPEND W3C_C_AOT_TESTS_NEEDS_LUA ${TEST_NUM})
        set(W3C_C_AOT_TESTS_NEEDS_LUA ${W3C_C_AOT_TESTS_NEEDS_LUA} PARENT_SCOPE)
    endif()
    if(_SWCT_NEEDS_DOM)
        list(APPEND W3C_C_AOT_TESTS_NEEDS_DOM ${TEST_NUM})
        set(W3C_C_AOT_TESTS_NEEDS_DOM ${W3C_C_AOT_TESTS_NEEDS_DOM} PARENT_SCOPE)
    endif()
    if(_SWCT_NEEDS_HTTP)
        list(APPEND W3C_C_AOT_TESTS_NEEDS_HTTP ${TEST_NUM})
        set(W3C_C_AOT_TESTS_NEEDS_HTTP ${W3C_C_AOT_TESTS_NEEDS_HTTP} PARENT_SCOPE)
    endif()

    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(TXML_FILE "${RESOURCE_DIR}/test${TEST_NUM}.txml")
    set(SCXML_FILE "${OUTPUT_DIR}/test${TEST_NUM}.scxml")
    set(GENERATED_HEADER "${OUTPUT_DIR}/test${TEST_NUM}_sm.h")
    set(GENERATED_SOURCE "${OUTPUT_DIR}/test${TEST_NUM}_sm.c")

    # Allow either `resources/${N}/test${N}.scxml` (already-converted SCXML)
    # or `resources/${N}/test${N}.txml` (TXML pending conversion). Most W3C
    # fixtures ship as TXML; the SCXML path catches a small set of hand-edited
    # documents (e.g. test513 in C++).
    set(RESOURCE_SCXML "${RESOURCE_DIR}/test${TEST_NUM}.scxml")
    set(RESOURCE_TXT "${RESOURCE_DIR}/test${TEST_NUM}.txt")
    set(_TXT_COPY_CMD)
    if(EXISTS "${RESOURCE_TXT}")
        # W3C SCXML 5.2.2: <data src="file:test${N}.txt"> reads its sidecar at
        # codegen time via the `read_data_src` filter (sce-build/src/filters.rs).
        # Mirror the C++ harness's behaviour (lines 195/220 above) by copying
        # the sidecar alongside the generated SCXML so the codegen base_path
        # resolves the file under ${OUTPUT_DIR}.
        set(_TXT_COPY_CMD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${RESOURCE_TXT}" "${OUTPUT_DIR}/test${TEST_NUM}.txt"
        )
    endif()
    if(EXISTS "${RESOURCE_SCXML}")
        add_custom_command(
            OUTPUT "${SCXML_FILE}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
            COMMAND ${CMAKE_COMMAND} -E copy "${RESOURCE_SCXML}" "${SCXML_FILE}"
            ${_TXT_COPY_CMD}
            DEPENDS "${RESOURCE_SCXML}"
            COMMENT "Using existing SCXML: test${TEST_NUM}.scxml (C11)"
            VERBATIM
        )
    elseif(EXISTS "${TXML_FILE}")
        add_custom_command(
            OUTPUT "${SCXML_FILE}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
            COMMAND txml-converter "${TXML_FILE}" "${SCXML_FILE}"
            COMMAND "${SCE_CODEGEN}" fix-scxml-name "${SCXML_FILE}" "test${TEST_NUM}"
            ${_TXT_COPY_CMD}
            DEPENDS txml-converter "${TXML_FILE}"
            COMMENT "Converting TXML to SCXML: test${TEST_NUM}.txml (C11)"
            VERBATIM
        )
    else()
        message(WARNING "Neither TXML nor SCXML found for test${TEST_NUM} — Skipping")
        return()
    endif()

    add_custom_command(
        OUTPUT "${GENERATED_HEADER}" "${GENERATED_SOURCE}"
        COMMAND "${SCE_CODEGEN}" generate "${SCXML_FILE}" -l c11 -o "${OUTPUT_DIR}"
        DEPENDS "${SCXML_FILE}" "${SCE_CODEGEN}"
        COMMENT "Generating C11 code: test${TEST_NUM}_sm.{h,c}"
        VERBATIM
    )

    list(APPEND GENERATED_W3C_C_HEADERS "${GENERATED_HEADER}")
    list(APPEND GENERATED_W3C_C_SOURCES "${GENERATED_SOURCE}")
    set(GENERATED_W3C_C_HEADERS ${GENERATED_W3C_C_HEADERS} PARENT_SCOPE)
    set(GENERATED_W3C_C_SOURCES ${GENERATED_W3C_C_SOURCES} PARENT_SCOPE)
endfunction()
