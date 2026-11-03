# SCE Static W3C Test Code Generation
# Generates C++ and C11 state machine code from the W3C SCXML test suite
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

# Which files a generation writes is asked of the generator, in one place for
# this route and the integration route alike.
include(${CMAKE_CURRENT_LIST_DIR}/SCECodegenPlan.cmake)

# sce_generate_aot_test_header: Generate AOT test header (TestXXX.h) from a template
#
# metadata.txt is the one source of a test's description, for the Interpreter
# and the AOT runner alike: AotTestBase::getDescription() reads it at run time.
# The generated header therefore keeps no copy of that text — a copy written
# here would be frozen at generation and drift from the file it was taken from.
# Nor does it state the section the test targets: that is `specnum` in the
# same file, and `sce-codegen check-aot-briefs` refuses a brief that states
# one. The template substitutes the test number alone.
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

    # Generate TestXXX.h from template
    configure_file(
        "${TEMPLATE_FILE}"
        "${AOT_TEST_HEADER}"
        @ONLY
    )

    message(STATUS "Generated AOT test header: Test${TEST_NUM}.h")
endfunction()

# _sce_generate_w3c_fixture: generate every machine one W3C fixture needs, in
#   one language
#
# A fixture is its committed document, `resources/<id>/test<id>.scxml`, and
# the child documents it starts. What its generation writes is not named here:
# it is asked of the generator (`sce_codegen_plan`), for the fixture's document
# and then for each child document that answer names, and every file of each
# answer is declared as an output of the command that writes it. A document
# that starts a child by `src`, a hybrid `<invoke>` with its stub, an inline
# `<invoke>` with the machine synthesized from it, a declared transition
# function with its header: each is in the answer because the generation
# writes it, so a declaration a document adds tomorrow is declared with it.
#
# ── Read at configure time, from the committed document ────────────────
#
# The plan has to be asked before anything is built, so the documents are
# read where they are committed, and staged into OUTPUT_DIR as the tree is
# configured (`configure_file`, so an edit reconfigures it, which is when the
# plan is asked again). Each document is planned in place, beside the child
# documents it starts, and generated from its staged copy: every file a
# generation writes lands in OUTPUT_DIR under the same name whichever copy it
# read, and the staged copy is the one whose directory the generated machine
# resolves a `<data src>` against at run time, which is why the sidecar
# `test<id>.txt` is staged beside it.
#
# A child document the fixture holds in RESOURCE_DIR is staged beside its
# parent before the parent is generated, and is an input of both: the parent
# reads it to learn what the child sends it, and its own machine is generated
# from it. That covers the author's sub-fixtures and the stubs of hybrid
# invokes, which the corpus tracks beside their parents
# (`hybrid_stubs_are_tracked_as_generated` holds each stub to the text the
# generation writes, so the parent's rewrite of its staged copy changes no
# byte). A child document the fixture does not hold is one only a generation
# writes, and its machine's outputs cannot be planned before the build has
# run, so it stops the configuration.
#
# ── One owner per generated file ──────────────────────────────────────
#
# Until 2026-08-13 the children were emitted by a generated CMake script the
# parent rule invoked, reading a list the generator wrote. That script was a
# second producer ninja could not see: its outputs were declared nowhere, and
# it re-emitted files that DID have rules, rewriting them after ninja had
# recorded those rules' depfiles, so eight rules regenerated on every build,
# forever. The hand-kept lists that replaced it were the next second copy:
# they named the files a fixture's generation wrote as the corpus stood, and a
# document declaring something new wrote a file no rule owned. Every file here
# now has the one command that writes it, and that command's declaration is
# the generator's own answer.
#
# Args:
#   TEST_NUM     The fixture id.
#   OUTPUT_DIR   Where the machines are written, shared by every fixture of
#                the language.
#   LANGUAGE     cpp or c11.
#   CHILD_ARGS   What a child document's generation is given beside what the
#                fixture's own is (the C++ child is generated `--as-child`).
#   FORMAT_ARGS  What every generation is given that changes bytes, never
#                which files.
#
# Sets in the caller's scope:
#   _W3C_FIXTURE_HEADERS  every header any of the fixture's generations writes
#   _W3C_FIXTURE_SOURCES  every C translation unit any of them writes
function(_sce_generate_w3c_fixture TEST_NUM OUTPUT_DIR LANGUAGE)
    cmake_parse_arguments(PARSE_ARGV 3 _FIX "" "" "CHILD_ARGS;FORMAT_ARGS")
    if(_FIX_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "_sce_generate_w3c_fixture(${TEST_NUM}): unexpected argument(s): "
            "${_FIX_UNPARSED_ARGUMENTS}")
    endif()

    # §synth-6.2.6 source-hash root. Every generation passes
    # `--input-root "${RESOURCE_DIR}"` because its input is STAGED into
    # OUTPUT_DIR, and OUTPUT_DIR is shared by every registered test. Without
    # the override the hash root defaults to the input file's parent — the
    # shared output directory — which is wrong twice over: the digest then
    # covers ~250 unrelated fixtures instead of this test's inputs, and the
    # walk reads a directory that sibling targets are concurrently writing
    # into, so `read_dir` can list a file that `fs::read` no longer finds and
    # the build dies on a race. `resources/${TEST_NUM}/` is tracked and no
    # build step writes to it, so the source set is quiescent by construction.
    # Same pattern as SCEStaticIntegrationFixture.cmake's FIXTURE_ROOT.
    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(_STEM "test${TEST_NUM}")

    # The committed document is what every backend generates from
    # (`sce-codegen generate-w3c` reads it and nothing else), and what this
    # route generates from. There is no second way to reach a fixture's SCXML
    # here: converting the TXML at build time was one, kept behind the
    # committed document after every registered fixture had one, and it ran
    # for no fixture.
    if(NOT EXISTS "${RESOURCE_DIR}/${_STEM}.scxml")
        message(FATAL_ERROR
            "W3C test ${TEST_NUM} is registered, and resources/${TEST_NUM}/${_STEM}.scxml "
            "is not committed. Every registered fixture is generated from its committed "
            "SCXML: convert its TXML (with any sub-fixture beside it) and commit the "
            "result, as docs/SCE_W3C_TEST_REGISTRATION.md Step 1 says.")
    endif()

    file(MAKE_DIRECTORY "${OUTPUT_DIR}")
    configure_file("${RESOURCE_DIR}/${_STEM}.scxml" "${OUTPUT_DIR}/${_STEM}.scxml" COPYONLY)
    # W3C SCXML 5.3.1: `<data src="file:test<id>.txt">` is read beside the
    # document, at generation and by the machine at run time.
    set(_SIDECARS "")
    if(EXISTS "${RESOURCE_DIR}/${_STEM}.txt")
        configure_file("${RESOURCE_DIR}/${_STEM}.txt" "${OUTPUT_DIR}/${_STEM}.txt" COPYONLY)
        list(APPEND _SIDECARS "${OUTPUT_DIR}/${_STEM}.txt")
    endif()

    set(_HEADERS "")
    set(_SOURCES "")
    set(_DECLARED "")
    set(_VISITED "")
    set(_PENDING "${_STEM}")
    list(LENGTH _PENDING _LEFT)
    while(_LEFT GREATER 0)
        list(POP_FRONT _PENDING _MACHINE)
        list(APPEND _VISITED "${_MACHINE}")

        if("${_MACHINE}" STREQUAL "${_STEM}")
            set(_ARGS "")
            set(_INPUTS "${OUTPUT_DIR}/${_MACHINE}.scxml" ${_SIDECARS})
        else()
            set(_ARGS ${_FIX_CHILD_ARGS})
            set(_INPUTS "${OUTPUT_DIR}/${_MACHINE}.scxml")
        endif()

        sce_codegen_plan(_PLAN
            STEM "${_MACHINE}"
            DOCUMENT "${RESOURCE_DIR}/${_MACHINE}.scxml"
            OUTPUT_DIR "${OUTPUT_DIR}"
            LANGUAGE ${LANGUAGE}
            INPUT_ROOT "${RESOURCE_DIR}"
            CHILDREN_HELD_IN "${RESOURCE_DIR}"
            ARGS ${_ARGS})

        # W3C SCXML 6.4: every child document a machine starts needs a machine
        # of its own, generated from the copy staged beside it.
        foreach(_CHILD ${_PLAN_CHILDREN})
            if(NOT "${OUTPUT_DIR}/${_CHILD}.scxml" IN_LIST _PLAN_STAGED)
                message(FATAL_ERROR
                    "W3C test ${TEST_NUM}: generating ${_MACHINE} (${LANGUAGE}) writes the "
                    "child document ${_CHILD}.scxml, which resources/${TEST_NUM}/ does not "
                    "hold, so the outputs of its own generation cannot be planned before "
                    "the build has run. A hybrid `<invoke>`'s stub is tracked beside its "
                    "parent: write it with UPDATE_EXPECT=1 cargo test -p sce-build --test "
                    "hybrid_stubs_are_tracked_as_generated, review it, and commit it.")
            endif()
            if(NOT "${_CHILD}" IN_LIST _VISITED AND NOT "${_CHILD}" IN_LIST _PENDING)
                list(APPEND _PENDING "${_CHILD}")
            endif()
        endforeach()

        # Two commands declaring one file is two producers of it, which is the
        # shape this route was rebuilt to remove.
        foreach(_FILE ${_PLAN_OUTPUTS} ${_PLAN_BYPRODUCTS})
            if("${_FILE}" IN_LIST _DECLARED)
                message(FATAL_ERROR
                    "W3C test ${TEST_NUM}: ${_FILE} is planned by two of its generations "
                    "(the second is ${_MACHINE}, ${LANGUAGE}); one file has one producer")
            endif()
            list(APPEND _DECLARED "${_FILE}")
        endforeach()

        # `--write-deps` + DEPFILE: the generated source depends on every
        # jinja2 template that rendered it, not just on the SCXML. Without it a
        # template edit leaves the artefact stale and the build reuses it
        # silently.
        add_custom_command(
            OUTPUT ${_PLAN_OUTPUTS}
            COMMAND "${SCE_CODEGEN}" generate "${OUTPUT_DIR}/${_MACHINE}.scxml"
                    -l ${LANGUAGE} -o "${OUTPUT_DIR}"
                    --input-root "${RESOURCE_DIR}"
                    --write-deps "${_PLAN_DEPFILE}"
                    ${_ARGS}
                    ${_FIX_FORMAT_ARGS}
            DEPENDS ${_INPUTS} ${_PLAN_STAGED} "${SCE_CODEGEN}"
            DEPFILE "${_PLAN_DEPFILE}"
            ${_PLAN_BYPRODUCTS_ARG}
            COMMENT "Generating ${LANGUAGE} code for W3C test ${TEST_NUM}: ${_MACHINE}"
            VERBATIM
        )

        list(APPEND _HEADERS ${_PLAN_HEADERS})
        list(APPEND _SOURCES ${_PLAN_SOURCES})
        list(LENGTH _PENDING _LEFT)
    endwhile()

    set(_W3C_FIXTURE_HEADERS "${_HEADERS}" PARENT_SCOPE)
    set(_W3C_FIXTURE_SOURCES "${_SOURCES}" PARENT_SCOPE)
endfunction()

# sce_generate_static_w3c_test: Generate C++ code for a single W3C test
#
# This does NOT create an executable: it declares the generation of the
# fixture's machines (`_sce_generate_w3c_fixture`) and appends every header
# they write to GENERATED_W3C_HEADERS, which `w3c_test_cli` lists so that each
# is generated before any unit including it is compiled.
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

    # 3. The machines. Formatted inside sce-codegen with the pinned
    # clang-format — see SCEClangFormat.cmake.
    sce_codegen_format_args(_FORMAT_ARGS)
    _sce_generate_w3c_fixture(${TEST_NUM} "${OUTPUT_DIR}" cpp
        CHILD_ARGS --as-child
        FORMAT_ARGS ${_FORMAT_ARGS})

    list(APPEND GENERATED_W3C_HEADERS ${_W3C_FIXTURE_HEADERS})
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
# RFC §5.J.1 sibling of `sce_generate_static_w3c_test`. Declares the
# generation of the fixture's machines with `sce-codegen -l c11`
# (`_sce_generate_w3c_fixture`), and sets
# `W3C_C_AOT_TEST_${TEST_NUM}_SOURCES` (PARENT_SCOPE) to every translation
# unit and header those generations write: the fixture's machine and each
# child's, synthesized from an inline `<invoke>` or generated from a child
# document of its own. The caller compiles them into the fixture binary; the
# parent's machine includes each child's header and embeds the child as a
# struct member (see `tools/codegen/templates/c/state_machine.h.jinja2`), and
# listing the headers makes each child's generation precede the compilation of
# the unit that includes it.
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
# There is no flag for a fixture that starts children: which children a
# fixture's machine starts is the generator's answer, and the sources above
# carry them whether or not the call says so.
function(sce_generate_static_w3c_c_test TEST_NUM OUTPUT_DIR)
    cmake_parse_arguments(_SWCT "NEEDS_LUA;NEEDS_DOM;NEEDS_HTTP" "" "" ${ARGN})
    if(_SWCT_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_generate_static_w3c_c_test(${TEST_NUM}): unexpected argument(s): "
            "${_SWCT_UNPARSED_ARGUMENTS}. NEEDS_INVOKE is gone: the children a "
            "fixture's machine starts are read from `sce-codegen --plan generate`, "
            "not flagged here. Remove it from the call.")
    endif()

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

    _sce_generate_w3c_fixture(${TEST_NUM} "${OUTPUT_DIR}" c11)

    set(W3C_C_AOT_TEST_${TEST_NUM}_SOURCES
        ${_W3C_FIXTURE_SOURCES} ${_W3C_FIXTURE_HEADERS} PARENT_SCOPE)
endfunction()
