# SCE Static W3C Test Code Generation
# Generates C++ state machine code from W3C SCXML test suite
#
# Requires sce-codegen (Rust binary).
# Build: cargo build --bin sce-codegen --features cli -p sce-build

# Set CMake policy CMP0116 to NEW (Ninja DEPFILE transformation)
# This suppresses warnings for add_custom_command DEPFILE usage
if(POLICY CMP0116)
    cmake_policy(SET CMP0116 NEW)
endif()

# clang-format integration for generated C++ code
include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

# Resolve the generator through the ecosystem locator rather than here.
#
# This module used to carry its own `find_program`, with the profiles in
# its own order, and that second copy cost twice. It listed debug before
# release while the root list's install rule listed them the other way, so
# one tree could build against a fresh debug binary and install a stale
# release one. And because it never reached SCEFindCodegen, it never
# reached the content witness that module runs — on the W3C static-test
# lane, which is exactly where a months-old generator was found generating
# on 2026-08-13.
#
# The duplicate hid from `codegen_binary_resolution.rs` because CMake
# spells the search across two arguments and that gate matched only the
# one-line shell spelling. `nothing_outside_the_locators_reaches_into_a_
# profile_directory` is the check that now sees this shape.
include(${CMAKE_CURRENT_LIST_DIR}/SCEFindCodegen.cmake)
message(STATUS "SCE: Using code generator: ${SCE_CODEGEN}")

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

    # §synth-6.2.6 source-hash root. Every codegen call below passes
    # `--input-root "${RESOURCE_DIR}"` because the inputs are STAGED into
    # OUTPUT_DIR, and OUTPUT_DIR is shared by every registered test. Without
    # the override the hash root defaults to the input file's parent — the
    # shared output directory — which is wrong twice over: the digest then
    # covers ~250 unrelated fixtures instead of this test's inputs, and the
    # walk reads a directory that sibling targets are concurrently staging
    # into, so `read_dir` can list a file that `fs::read` no longer finds and
    # the build dies on a race. `resources/${TEST_NUM}/` is tracked and no
    # build step writes to it, so the source set is quiescent by construction.
    # Same pattern as SCEStaticIntegrationFixture.cmake's FIXTURE_ROOT.
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
            COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${SUB_SCXML_FILE}" -l cpp -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --as-child --write-deps "${SUB_HEADER_FILE}.d"
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

    # Step 2: SCXML -> C++ code generation (parent + children)
    #
    # ── One owner per generated file ──────────────────────────────────
    #
    # Until 2026-08-13 the children were emitted by a generated CMake script
    # the parent rule invoked (`process_children_<N>.cmake`), reading a
    # `_children.txt` the generator wrote. That script was a second producer
    # ninja could not see: its outputs were declared nowhere, so nothing
    # rebuilt them when their inputs changed, and it re-emitted files that
    # DID have rules — rewriting them after ninja had recorded those rules'
    # depfiles. `ninja -d explain` reported `stored deps info out of date`
    # and eight rules regenerated on every build, forever, forcing a relink
    # whose binary differed in 40 bytes with every object byte-identical.
    #
    # The three child kinds have three different natural owners, and once
    # each is named the script has nothing left to do:
    #
    #   `test<N>sub*`              own rule (above) — an author-supplied
    #                              sibling fixture, reached by
    #                              `<invoke src="file:...">`.
    #   `__sce_synth_invoke__*`    THE PARENT RULE. The parent's own
    #                              generation materialises these; measured,
    #                              `generate test239.scxml` emits
    #                              `test239__sce_synth_invoke__invoke_1_sm.h`
    #                              beside its own header. So they are its
    #                              byproducts, not a separate step's output.
    #   `test<N>_hybrid*`          own rule (below). Measured: the parent
    #                              does NOT emit these — `generate
    #                              test216.scxml` produces `test216_sm.h`
    #                              alone — so they are the one kind that
    #                              genuinely needs a producer of its own.
    #
    # All three are discoverable at configure time from RESOURCE_DIR, which
    # is what makes declaring them possible: the synth-invoke and hybrid
    # documents are committed there (`write_if_changed` in sce-build keeps
    # them stable), exactly as the C11 path below has always relied on.
    #
    # W3C SCXML 6.2/6.4: the parent header depends on child headers.

    # Synth-invoke children: byproducts of the parent's own generation.
    # Declared so ninja knows who makes them and stops treating them as
    # files that appeared from nowhere.
    file(GLOB _CPP_SYNTH_FILES
        "${RESOURCE_DIR}/test${TEST_NUM}__sce_synth_invoke__*.scxml")
    set(_CPP_PARENT_BYPRODUCTS "")
    foreach(_CPP_SYNTH ${_CPP_SYNTH_FILES})
        get_filename_component(_CPP_SYNTH_NAME "${_CPP_SYNTH}" NAME_WE)
        list(APPEND _CPP_PARENT_BYPRODUCTS
            "${OUTPUT_DIR}/${_CPP_SYNTH_NAME}_sm.h"
            "${OUTPUT_DIR}/${_CPP_SYNTH_NAME}_sm.inl")
    endforeach()

    # Hybrid `<invoke srcexpr=...>` stubs: the one child kind with no other
    # producer. Staged then generated, mirroring the sub-fixture rules above
    # and the C11 path below, so it too is a declared output with a depfile.
    file(GLOB _CPP_HYBRID_FILES "${RESOURCE_DIR}/test${TEST_NUM}_hybrid*.scxml")
    foreach(_CPP_HYBRID ${_CPP_HYBRID_FILES})
        get_filename_component(_CPP_HYBRID_NAME "${_CPP_HYBRID}" NAME_WE)
        set(_CPP_HYBRID_SCXML "${OUTPUT_DIR}/${_CPP_HYBRID_NAME}.scxml")
        set(_CPP_HYBRID_HEADER "${OUTPUT_DIR}/${_CPP_HYBRID_NAME}_sm.h")
        set(_CPP_HYBRID_INL "${OUTPUT_DIR}/${_CPP_HYBRID_NAME}_sm.inl")

        add_custom_command(
            OUTPUT "${_CPP_HYBRID_SCXML}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
            COMMAND ${CMAKE_COMMAND} -E copy "${_CPP_HYBRID}" "${_CPP_HYBRID_SCXML}"
            DEPENDS "${_CPP_HYBRID}"
            COMMENT "Staging hybrid child SCXML: ${_CPP_HYBRID_NAME}.scxml"
            VERBATIM
        )
        add_custom_command(
            OUTPUT "${_CPP_HYBRID_HEADER}"
            COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${_CPP_HYBRID_SCXML}" -l cpp -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --as-child --write-deps "${_CPP_HYBRID_HEADER}.d"
            DEPENDS "${_CPP_HYBRID_SCXML}" "${SCE_CODEGEN}"
            DEPFILE "${_CPP_HYBRID_HEADER}.d"
            BYPRODUCTS "${_CPP_HYBRID_INL}"
            COMMENT "Generating C++ code: ${_CPP_HYBRID_NAME}_sm.h + .inl"
            VERBATIM
        )

        list(APPEND SUB_HEADER_DEPENDENCIES "${_CPP_HYBRID_HEADER}")
        list(APPEND GENERATED_W3C_HEADERS "${_CPP_HYBRID_HEADER}")
        set(GENERATED_W3C_HEADERS ${GENERATED_W3C_HEADERS} PARENT_SCOPE)
    endforeach()

    # clang-format post-processing (no-op if not available)
    if(SCE_CLANG_FORMAT_FOUND)
        set(_PARENT_FORMAT_CMD COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}" -i "${GENERATED_HEADER}" "${GENERATED_INL}")
    else()
        set(_PARENT_FORMAT_CMD "")
    endif()
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}"
        COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${SCXML_FILE}" -l cpp -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --write-deps "${GENERATED_HEADER}.d"
        ${_PARENT_FORMAT_CMD}
        DEPENDS "${SCXML_FILE}" ${SUB_HEADER_DEPENDENCIES} "${SCE_CODEGEN}"
        DEPFILE "${GENERATED_HEADER}.d"
        BYPRODUCTS "${GENERATED_INL}" ${_CPP_PARENT_BYPRODUCTS}
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
# pair under OUTPUT_DIR via `sce-codegen -l c11`. When the fixture carries
# `<invoke>` elements, sce-build pre-emits the inline `<content><scxml>...`
# children as separate `test${N}__sce_synth_invoke__*.scxml` files
# alongside `resources/${N}/test${N}.scxml` (parser §9.6.6 rule 3 default
# = local extraction). The function discovers those siblings via
# `file(GLOB)` and emits a per-child `_sm.{h,c}` pair — the parent SM
# `_sm.c` includes each child's `_sm.h` and embeds the child as a struct
# member (see `tools/codegen/templates/c/state_machine.h.jinja2`).
#
# Optional flag NEEDS_LUA: append the test to W3C_C_AOT_TESTS_NEEDS_LUA so
# the caller can conditionally `target_link_libraries(... lua54)` only for
# fixtures whose codegen embedded `luaL_dostring` calls. Datamodel-less
# fixtures (e.g. test355, test144) leave Lua out of the link entirely so
# the MCU footprint stays minimal for the downstream consumer.
#
# Optional flag NEEDS_DOM: append the test to W3C_C_AOT_TESTS_NEEDS_DOM so
# the caller can link the host-side XML DOM helper (backends/c/tests/support/
# dom.c + lua_dom_binding.c) alongside Lua. NEEDS_DOM implies NEEDS_LUA.
# Currently used only by the W3C SCXML B.2 corpus (test557, test561) —
# the helper is testbench-only and never enters the backends/c/runtime link.
#
# Optional flag NEEDS_HTTP: append the test to W3C_C_AOT_TESTS_NEEDS_HTTP so
# the caller can link the host-side HTTP/1.1 client + JSON extractor
# (backends/c/tests/support/http_client.c). Used by the W3C SCXML C.2
# BasicHTTPEventProcessor corpus that issues a real HTTP POST against the
# Node.js standalone server (`tests/w3c/standalone_http_server.js`).
# Independent of NEEDS_LUA / NEEDS_DOM — combined freely as the fixture
# requires.
#
# Optional flag NEEDS_INVOKE: append the test to W3C_C_AOT_TESTS_NEEDS_INVOKE
# so the caller can iterate the per-test child source list set by this
# function (`W3C_C_AOT_TEST_${TEST_NUM}_CHILD_SOURCES`) and link those
# child `_sm.c` translation units into the fixture binary. The parent's
# generated `_sm.h` already pulls in the child's `_sm.h` via
# `state_machine.h.jinja2` so child types are visible in the parent's
# struct. Independent of the other NEEDS_* flags — combined freely.
#
function(sce_generate_static_w3c_c_test TEST_NUM OUTPUT_DIR)
    cmake_parse_arguments(_SWCT "NEEDS_LUA;NEEDS_DOM;NEEDS_HTTP;NEEDS_INVOKE" "" "" ${ARGN})

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
    if(_SWCT_NEEDS_INVOKE)
        list(APPEND W3C_C_AOT_TESTS_NEEDS_INVOKE ${TEST_NUM})
        set(W3C_C_AOT_TESTS_NEEDS_INVOKE ${W3C_C_AOT_TESTS_NEEDS_INVOKE} PARENT_SCOPE)
    endif()

    # §synth-6.2.6 source-hash root. Every codegen call below passes
    # `--input-root "${RESOURCE_DIR}"` because the inputs are STAGED into
    # OUTPUT_DIR, and OUTPUT_DIR is shared by every registered test. Without
    # the override the hash root defaults to the input file's parent — the
    # shared output directory — which is wrong twice over: the digest then
    # covers ~250 unrelated fixtures instead of this test's inputs, and the
    # walk reads a directory that sibling targets are concurrently staging
    # into, so `read_dir` can list a file that `fs::read` no longer finds and
    # the build dies on a race. `resources/${TEST_NUM}/` is tracked and no
    # build step writes to it, so the source set is quiescent by construction.
    # Same pattern as SCEStaticIntegrationFixture.cmake's FIXTURE_ROOT.
    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(TXML_FILE "${RESOURCE_DIR}/test${TEST_NUM}.txml")
    set(SCXML_FILE "${OUTPUT_DIR}/test${TEST_NUM}.scxml")
    set(GENERATED_HEADER "${OUTPUT_DIR}/test${TEST_NUM}_sm.h")
    set(GENERATED_SOURCE "${OUTPUT_DIR}/test${TEST_NUM}_sm.c")

    # W3C SCXML 6.4: discover synthesised invoke children. sce-build extracts
    # inline `<content><scxml>...</scxml></content>` into sibling files at
    # parse time (see parser.rs §9.6.6 rule 3) so each child is statically
    # codegen-able. The pattern matches `test${N}__sce_synth_invoke__*.scxml`;
    # `file(GLOB)` runs at configure time, so the parent's add_executable
    # below sees all child sources without a re-glob trigger.
    # Synth-invoke children are the PARENT's own output, so they are named as
    # its byproducts and given no rules of their own.
    #
    # They used to have two: a staging copy of the committed `.scxml` and a
    # generate rule for the SM. Measured 2026-08-14 by running the generator
    # on `resources/247/test247.scxml` into an empty directory — one parent
    # invocation writes `test247__sce_synth_invoke__invoke_0.scxml`,
    # `_sm.h` AND `_sm.c`. Every one of those three was also the OUTPUT of a
    # rule here, and because the parent DEPENDS on the child headers it runs
    # LAST and rewrites them after ninja has recorded the child rule's deps.
    # The result was 41 generated headers reporting `stored deps info out of
    # date` on every build, forever — the same shape
    # `build_writes_and_undeclared_outputs` removed from the C++ tree, left
    # behind on this one. The cpp path above already carries the fix; this is
    # it in C11's alphabet, where the difference is that a child here is a
    # `.c` to compile rather than a header to include.
    file(GLOB _SYNTH_INVOKE_FILES
        "${RESOURCE_DIR}/test${TEST_NUM}__sce_synth_invoke__*.scxml")
    set(_CHILD_GENERATED_SOURCES "")
    set(_C_PARENT_BYPRODUCTS "")
    foreach(_SYNTH_FILE ${_SYNTH_INVOKE_FILES})
        get_filename_component(_CHILD_NAME "${_SYNTH_FILE}" NAME_WE)
        list(APPEND _C_PARENT_BYPRODUCTS
            "${OUTPUT_DIR}/${_CHILD_NAME}.scxml"
            "${OUTPUT_DIR}/${_CHILD_NAME}_sm.h"
            "${OUTPUT_DIR}/${_CHILD_NAME}_sm.c")
        # Still compiled into the test binary. A byproduct is marked
        # GENERATED and carries the producing edge, and the executable
        # already lists the parent's `_sm.c` — the one invocation that makes
        # both — so the ordering is the build graph's rather than a comment's.
        list(APPEND _CHILD_GENERATED_SOURCES "${OUTPUT_DIR}/${_CHILD_NAME}_sm.c")
    endforeach()

    # W3C SCXML 6.4 (test226/276): discover author-supplied sibling sub-fixture
    # TXML files. These are referenced by the parent via `<invoke src="file:test${N}sub1.scxml">`
    # rather than inline `<content>`, so sce-build sees them as external paths
    # and the parent's generated `_sm.h` `#include`s the child's `_sm.h`.
    # Mirrors the C++ path's `test${N}sub*.txml` discovery (line 137 above) —
    # convert TXML→SCXML, then codegen the C11 SM. Glob runs at configure
    # time so the parent's add_executable below sees the staged files.
    file(GLOB _SUB_TXML_FILES "${RESOURCE_DIR}/test${TEST_NUM}sub*.txml")
    foreach(_SUB_TXML ${_SUB_TXML_FILES})
        get_filename_component(_SUB_NAME "${_SUB_TXML}" NAME_WE)
        set(_SUB_SCXML "${OUTPUT_DIR}/${_SUB_NAME}.scxml")
        set(_SUB_HEADER "${OUTPUT_DIR}/${_SUB_NAME}_sm.h")
        set(_SUB_SOURCE "${OUTPUT_DIR}/${_SUB_NAME}_sm.c")

        add_custom_command(
            OUTPUT "${_SUB_SCXML}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
            COMMAND txml-converter "${_SUB_TXML}" "${_SUB_SCXML}"
            DEPENDS txml-converter "${_SUB_TXML}"
            COMMENT "Converting ${_SUB_NAME}.txml to SCXML (C11 sub-fixture)"
            VERBATIM
        )
        add_custom_command(
            OUTPUT "${_SUB_HEADER}" "${_SUB_SOURCE}"
            COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${_SUB_SCXML}" -l c11 -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --write-deps "${_SUB_SOURCE}.d"
            DEPENDS "${_SUB_SCXML}" "${SCE_CODEGEN}"
            DEPFILE "${_SUB_SOURCE}.d"
            COMMENT "Generating C11 code: ${_SUB_NAME}_sm.{h,c}"
            VERBATIM
        )

        list(APPEND _CHILD_GENERATED_SOURCES "${_SUB_SOURCE}")
    endforeach()

    # W3C SCXML 6.4 (test216/530): hybrid `<invoke srcexpr=...>` /
    # `<invoke><content expr=...>` children. sce-build writes an
    # immediate-final stub to RESOURCE_DIR for every hybrid invoke
    # (see `generate_hybrid_child_scxmls`'s c11 backend dest in
    # `sce-codegen`). The stubs follow `test${N}_hybrid*.scxml` —
    # picked up by the configure-time GLOB below alongside the
    # synth-invoke siblings. Each stub is staged into OUTPUT_DIR and
    # codegen'd as a child SM the same way inline-content children
    # are handled, so the parent's `_sm.h` `#include` resolves and the
    # hybrid arm in `invoke_methods.jinja2`'s `execute_pending_invokes`
    # has a real symbol to call into. The stubs are checked into the
    # parent's resource directory under git on first run — `write_if_changed`
    # in sce-build keeps the file stable across rebuilds.
    file(GLOB _HYBRID_STUB_FILES "${RESOURCE_DIR}/test${TEST_NUM}_hybrid*.scxml")
    foreach(_HYBRID_STUB ${_HYBRID_STUB_FILES})
        get_filename_component(_HYBRID_NAME "${_HYBRID_STUB}" NAME_WE)
        set(_HYBRID_SCXML "${OUTPUT_DIR}/${_HYBRID_NAME}.scxml")
        set(_HYBRID_HEADER "${OUTPUT_DIR}/${_HYBRID_NAME}_sm.h")
        set(_HYBRID_SOURCE "${OUTPUT_DIR}/${_HYBRID_NAME}_sm.c")

        add_custom_command(
            OUTPUT "${_HYBRID_SCXML}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${OUTPUT_DIR}"
            COMMAND ${CMAKE_COMMAND} -E copy "${_HYBRID_STUB}" "${_HYBRID_SCXML}"
            DEPENDS "${_HYBRID_STUB}"
            COMMENT "Staging hybrid invoke stub: ${_HYBRID_NAME}.scxml (C11)"
            VERBATIM
        )
        add_custom_command(
            OUTPUT "${_HYBRID_HEADER}" "${_HYBRID_SOURCE}"
            COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${_HYBRID_SCXML}" -l c11 -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --write-deps "${_HYBRID_SOURCE}.d"
            DEPENDS "${_HYBRID_SCXML}" "${SCE_CODEGEN}"
            DEPFILE "${_HYBRID_SOURCE}.d"
            COMMENT "Generating C11 code: ${_HYBRID_NAME}_sm.{h,c}"
            VERBATIM
        )

        list(APPEND _CHILD_GENERATED_SOURCES "${_HYBRID_SOURCE}")
    endforeach()

    set(W3C_C_AOT_TEST_${TEST_NUM}_CHILD_SOURCES "${_CHILD_GENERATED_SOURCES}" PARENT_SCOPE)

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

    # Collect child header outputs so the parent's codegen depends on them
    # — when the parent's `_sm.h` `#include`s the child's `_sm.h` (driven
    # by `state_machine.h.jinja2` when `model.invokes | scxml_family` is
    # non-empty), the build graph must materialise the child header
    # before the parent compilation that consumes it. Equally important
    # for `collect_child_to_parent_events` — sce-build's parser scans the
    # child SCXML at parent codegen time to populate
    # `child_has_send_to_parent` / `child_datamodel_vars`, so the child
    # SCXML must exist at the OUTPUT_DIR path before parent codegen runs.
    # Both synth-invoke (inline) and sibling-sub (test226/276 author-
    # supplied) shapes are tracked here so neither shape silently regens
    # the parent against a stale or absent child model.
    # Synth children are deliberately absent from this list: the parent makes
    # them, so depending on them would be the parent waiting for itself.
    set(_CHILD_HEADER_DEPS "")
    foreach(_SUB_TXML ${_SUB_TXML_FILES})
        get_filename_component(_SUB_NAME "${_SUB_TXML}" NAME_WE)
        list(APPEND _CHILD_HEADER_DEPS "${OUTPUT_DIR}/${_SUB_NAME}_sm.h")
    endforeach()
    foreach(_HYBRID_STUB ${_HYBRID_STUB_FILES})
        get_filename_component(_HYBRID_NAME "${_HYBRID_STUB}" NAME_WE)
        list(APPEND _CHILD_HEADER_DEPS "${OUTPUT_DIR}/${_HYBRID_NAME}_sm.h")
    endforeach()

    # The keyword only when there is something to name: a bare `BYPRODUCTS`
    # with no values is a configure error, and most fixtures carry no
    # synth-invoke child at all.
    set(_C_PARENT_BYPRODUCTS_ARG "")
    if(_C_PARENT_BYPRODUCTS)
        set(_C_PARENT_BYPRODUCTS_ARG BYPRODUCTS ${_C_PARENT_BYPRODUCTS})
    endif()
    add_custom_command(
        OUTPUT "${GENERATED_HEADER}" "${GENERATED_SOURCE}"
        COMMAND ${SCE_CODEGEN_ENV} "${SCE_CODEGEN}" generate "${SCXML_FILE}" -l c11 -o "${OUTPUT_DIR}" --input-root "${RESOURCE_DIR}" --write-deps "${GENERATED_SOURCE}.d"
        DEPENDS "${SCXML_FILE}" "${SCE_CODEGEN}" ${_CHILD_HEADER_DEPS}
        DEPFILE "${GENERATED_SOURCE}.d"
        ${_C_PARENT_BYPRODUCTS_ARG}
        COMMENT "Generating C11 code: test${TEST_NUM}_sm.{h,c}"
        VERBATIM
    )

    list(APPEND GENERATED_W3C_C_HEADERS "${GENERATED_HEADER}")
    list(APPEND GENERATED_W3C_C_SOURCES "${GENERATED_SOURCE}")
    set(GENERATED_W3C_C_HEADERS ${GENERATED_W3C_C_HEADERS} PARENT_SCOPE)
    set(GENERATED_W3C_C_SOURCES ${GENERATED_W3C_C_SOURCES} PARENT_SCOPE)
endfunction()
