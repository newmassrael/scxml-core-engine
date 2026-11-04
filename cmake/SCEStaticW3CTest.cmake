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

# Which fixtures a conformance route registers, and the harness of each, is
# read from the conformance registry, in one place for the C++ route and the
# C11 route alike (`sce_w3c_registry_fixtures`).
include(${CMAKE_CURRENT_LIST_DIR}/SCEW3CRegistry.cmake)

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
# and, in the same answer, for the generation of every child document that
# generation writes and of every child document those write in turn
# (`CHILD_GENERATIONS`), and every file of each answer is declared as an
# output of the command that writes it. A document that starts a child by
# `src`, a hybrid `<invoke>` with its stub, an inline `<invoke>` with the
# machine synthesized from it, a declared transition function with its
# header: each is in the answer because the generation writes it, so a
# declaration a document adds tomorrow is declared with it. This is the
# integration route's mechanism (SCEStaticIntegrationFixture.cmake), and
# neither route names a child document by hand.
#
# ── Read at configure time, from the committed document ────────────────
#
# The plan has to be asked before anything is built, so the fixture's
# document is read where it is committed, and staged into OUTPUT_DIR as the
# tree is configured (`configure_file`, so an edit reconfigures it, which is
# when the plan is asked again). It is planned in place, beside the child
# documents it starts, and generated from its staged copy: every file a
# generation writes lands in OUTPUT_DIR under the same name whichever copy it
# read, and the staged copy is the one whose directory the generated machine
# resolves a `<data src>` against at run time, which is why the sidecar
# `test<id>.txt` is staged beside it.
#
# A child document the author wrote beside the fixture (a sub-fixture started
# by `src`) is staged beside its parent before the parent is generated
# (`CHILDREN_HELD_IN`), and is an input of both: the parent reads it to learn
# what the child sends it, and its own machine is generated from it. A child
# document only a generation writes (the stub of a hybrid `<invoke>`) is that
# generation's byproduct, and its own generation is planned by the generator
# from the text the parent's generation would write, so nothing of it is
# tracked beside the fixture and its generation waits for its parent's.
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
#   FORMAT_ARGS  What every generation is given that changes bytes, never
#                which files.
#
# A child document's generation is the one the plan holds for it: `generate
# <child>` with the arguments the plan states for it (`<P>_ARGS`, `--as-child`
# for every child a fixture starts), and the fixture's language, output
# directory and input root. `--as-child` changes no C11 file (a C11 child
# detects what it sends its parent from its document), so both languages run
# the generation the plan planned.
#
# Sets in the caller's scope:
#   _W3C_FIXTURE_HEADERS  every header any of the fixture's generations writes
#   _W3C_FIXTURE_SOURCES  every C translation unit any of them writes
#   _W3C_FIXTURE_REQUIRES what the machines of any of them need linked, as the
#                         generator states it (`<PREFIX>_REQUIRES`), each once
function(_sce_generate_w3c_fixture TEST_NUM OUTPUT_DIR LANGUAGE)
    cmake_parse_arguments(PARSE_ARGV 3 _FIX "" "" "FORMAT_ARGS")
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

    # The fixture's generation, and, in the same answer, the generation of
    # every child document it writes (W3C SCXML 6.4: every child document a
    # machine starts needs a machine of its own).
    sce_codegen_plan(_PLAN
        STEM "${_STEM}"
        DOCUMENT "${RESOURCE_DIR}/${_STEM}.scxml"
        OUTPUT_DIR "${OUTPUT_DIR}"
        LANGUAGE ${LANGUAGE}
        INPUT_ROOT "${RESOURCE_DIR}"
        CHILDREN_HELD_IN "${RESOURCE_DIR}"
        CHILD_GENERATIONS)

    # Two commands declaring one file is two producers of it, which is the
    # shape this route was rebuilt to remove.
    set(_DECLARED ${_PLAN_OUTPUTS} ${_PLAN_BYPRODUCTS})

    # `--write-deps` + DEPFILE: the generated source depends on every jinja2
    # template that rendered it, not just on the SCXML. Without it a template
    # edit leaves the artefact stale and the build reuses it silently.
    add_custom_command(
        OUTPUT ${_PLAN_OUTPUTS}
        COMMAND "${SCE_CODEGEN}" generate "${OUTPUT_DIR}/${_STEM}.scxml"
                -l ${LANGUAGE} -o "${OUTPUT_DIR}"
                --input-root "${RESOURCE_DIR}"
                --write-deps "${_PLAN_DEPFILE}"
                ${_FIX_FORMAT_ARGS}
        DEPENDS "${OUTPUT_DIR}/${_STEM}.scxml" ${_SIDECARS} ${_PLAN_STAGED} "${SCE_CODEGEN}"
        DEPFILE "${_PLAN_DEPFILE}"
        ${_PLAN_BYPRODUCTS_ARG}
        COMMENT "Generating ${LANGUAGE} code for W3C test ${TEST_NUM}: ${_STEM}"
        VERBATIM
    )
    set(_HEADERS ${_PLAN_HEADERS})
    set(_SOURCES ${_PLAN_SOURCES})
    set(_REQUIRES ${_PLAN_REQUIRES})

    # Each child document the fixture's generation writes or stages, and each
    # one those write in turn, generated by a command of its own declared from
    # the plan the fixture's answer holds for it. It waits for the generation
    # that writes the document it reads.
    foreach(_CHILD ${_PLAN_GENERATIONS})
        sce_codegen_plan(_CHILD_PLAN FROM _PLAN CHILD "${_CHILD}")
        foreach(_FILE ${_CHILD_PLAN_OUTPUTS} ${_CHILD_PLAN_BYPRODUCTS})
            if("${_FILE}" IN_LIST _DECLARED)
                message(FATAL_ERROR
                    "W3C test ${TEST_NUM}: ${_FILE} is planned by two of its generations "
                    "(the second is ${_CHILD}, ${LANGUAGE}); one file has one producer")
            endif()
            list(APPEND _DECLARED "${_FILE}")
        endforeach()
        add_custom_command(
            OUTPUT ${_CHILD_PLAN_OUTPUTS}
            COMMAND "${SCE_CODEGEN}" generate "${_CHILD_PLAN_DOCUMENT}"
                    ${_CHILD_PLAN_ARGS}
                    -l ${LANGUAGE} -o "${OUTPUT_DIR}"
                    --input-root "${RESOURCE_DIR}"
                    --write-deps "${_CHILD_PLAN_DEPFILE}"
                    ${_FIX_FORMAT_ARGS}
            DEPENDS ${_CHILD_PLAN_DEPENDS} "${SCE_CODEGEN}"
            DEPFILE "${_CHILD_PLAN_DEPFILE}"
            ${_CHILD_PLAN_BYPRODUCTS_ARG}
            COMMENT "Generating ${LANGUAGE} code for W3C test ${TEST_NUM}: ${_CHILD}"
            VERBATIM
        )
        list(APPEND _HEADERS ${_CHILD_PLAN_HEADERS})
        list(APPEND _SOURCES ${_CHILD_PLAN_SOURCES})
        list(APPEND _REQUIRES ${_CHILD_PLAN_REQUIRES})
    endforeach()
    if(_REQUIRES)
        list(REMOVE_DUPLICATES _REQUIRES)
    endif()

    set(_W3C_FIXTURE_HEADERS "${_HEADERS}" PARENT_SCOPE)
    set(_W3C_FIXTURE_SOURCES "${_SOURCES}" PARENT_SCOPE)
    set(_W3C_FIXTURE_REQUIRES "${_REQUIRES}" PARENT_SCOPE)
endfunction()

# sce_prefetch_static_w3c_tests(<OUTPUT_DIR> <LANGUAGE> <TEST_NUM>...)
#
#   Ask the generator for the plan of every fixture named, in one batch,
#   before the fixtures are declared one by one: the plan of each is the one
#   `_sce_generate_w3c_fixture` asks, with the same options, so each
#   declaration then reads its answer where the batch kept it
#   (`sce_codegen_plan_request`, cmake/SCECodegenPlan.cmake). A fixture whose
#   answer is kept and current is not asked again.
#
#   A caller that knows the fixtures before it declares them calls this
#   first; the declaration asks the plan of a fixture that was not asked
#   ahead itself, so leaving one out costs time and changes no answer.
function(sce_prefetch_static_w3c_tests OUTPUT_DIR LANGUAGE)
    foreach(TEST_NUM IN LISTS ARGN)
        _sce_request_w3c_fixture(${TEST_NUM} "${OUTPUT_DIR}" ${LANGUAGE})
    endforeach()
    sce_codegen_plan_prefetch()
endfunction()

# _sce_request_w3c_fixture(<TEST_NUM> <OUTPUT_DIR> <LANGUAGE>)
#
#   Queue the request of the plan `_sce_generate_w3c_fixture` asks for one
#   fixture, for the next `sce_codegen_plan_prefetch` to send.
function(_sce_request_w3c_fixture TEST_NUM OUTPUT_DIR LANGUAGE)
    set(RESOURCE_DIR "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}")
    set(_STEM "test${TEST_NUM}")
    # `_sce_generate_w3c_fixture`'s options, word for word
    # (`the_conformance_cmake_route_declares_what_the_generator_plans`
    # holds the two to each other).
    sce_codegen_plan_request(
        STEM "${_STEM}"
        DOCUMENT "${RESOURCE_DIR}/${_STEM}.scxml"
        OUTPUT_DIR "${OUTPUT_DIR}"
        LANGUAGE ${LANGUAGE}
        INPUT_ROOT "${RESOURCE_DIR}"
        CHILDREN_HELD_IN "${RESOURCE_DIR}"
        CHILD_GENERATIONS)
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

# sce_generate_static_w3c_c_test: Register a W3C test generated as C11
#
# RFC §5.J.1 sibling of `sce_generate_static_w3c_test`. Registers the
# fixture; the one `sce_declare_registered_static_w3c_c_tests()` that follows
# the last registration asks the generator for every registered fixture's
# plan in one batch, then declares the generation of each fixture's machines
# with `sce-codegen -l c11` (`_sce_generate_w3c_fixture`) and sets
# `W3C_C_AOT_TEST_${TEST_NUM}_SOURCES` (PARENT_SCOPE) to every translation
# unit and header those generations write: the fixture's machine and each
# child's, synthesized from an inline `<invoke>` or generated from a child
# document of its own. The caller compiles them into the fixture binary; the
# parent's machine includes each child's header and embeds the child as a
# struct member (see `tools/codegen/templates/c/state_machine.h.jinja2`), and
# listing the headers makes each child's generation precede the compilation of
# the unit that includes it.
#
# The same declaration sets `W3C_C_AOT_TEST_${TEST_NUM}_REQUIRES` to what the
# machines of those generations need linked, as the generator states it
# (`--plan-requires`, `<PREFIX>_REQUIRES` in cmake/SCECodegenPlan.cmake): `lua`
# for a machine that embeds the Lua engine, `dom` for one that can hand its
# data model an XML document, `http` for one that sends over BasicHTTP. The
# caller links each with the library the runtime names for it
# (`sce_c_runtime_libraries`, backends/c/runtime/CMakeLists.txt), so a machine
# with no data model links neither Lua nor the DOM binding and keeps the
# freestanding profile of the downstream MCU consumer.
#
# The call takes no flag. Which fixtures a route registers is the registry's
# answer (`sce_w3c_registry_fixtures`), which children a fixture's machine
# starts and what each machine needs linked are the generator's, and none of
# them is restated beside an id: the NEEDS_INVOKE flag, then the NEEDS_LUA,
# NEEDS_DOM and NEEDS_HTTP flags on 202 hand-written calls, were such copies,
# and NEEDS_DOM had stopped selecting anything before it went.
#
# A fixture registered after the declaration, and a registration no
# declaration follows, are refused: either would be a fixture the build never
# generates while its runner is still named.
function(sce_generate_static_w3c_c_test TEST_NUM OUTPUT_DIR)
    list(LENGTH ARGN _EXTRA)
    if(_EXTRA GREATER 0)
        message(FATAL_ERROR
            "sce_generate_static_w3c_c_test(${TEST_NUM}): unexpected argument(s): "
            "${ARGN}. The call takes no flag: what a fixture's machines need linked is "
            "read from `sce-codegen --plan --plan-requires generate` "
            "(W3C_C_AOT_TEST_<id>_REQUIRES), and the children they start from the same "
            "plan. Remove it from the call.")
    endif()
    get_property(_DECLARED GLOBAL PROPERTY _SCE_W3C_C_DECLARED)
    if(_DECLARED)
        message(FATAL_ERROR
            "sce_generate_static_w3c_c_test(${TEST_NUM}) comes after "
            "sce_declare_registered_static_w3c_c_tests(), which declared the fixtures "
            "registered before it: this one would never be generated. Register it "
            "before that call.")
    endif()
    get_property(_REGISTERED GLOBAL PROPERTY _SCE_W3C_C_REGISTERED)
    if("${TEST_NUM}" IN_LIST _REGISTERED)
        message(FATAL_ERROR
            "sce_generate_static_w3c_c_test(${TEST_NUM}) registers a fixture registered "
            "already")
    endif()
    if(NOT _REGISTERED)
        # The first registration: the configure ends refusing a registration
        # no declaration followed.
        cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}"
            CALL _sce_refuse_undeclared_static_w3c_c_tests)
    endif()
    set_property(GLOBAL APPEND PROPERTY _SCE_W3C_C_REGISTERED "${TEST_NUM}")
    set_property(GLOBAL PROPERTY "_SCE_W3C_C_${TEST_NUM}_OUTPUT_DIR" "${OUTPUT_DIR}")

    # Accumulate test number into W3C_C_AOT_TESTS so the caller can iterate.
    list(APPEND W3C_C_AOT_TESTS ${TEST_NUM})
    set(W3C_C_AOT_TESTS ${W3C_C_AOT_TESTS} PARENT_SCOPE)
endfunction()

# sce_declare_registered_static_w3c_c_tests()
#
#   Declare every fixture `sce_generate_static_w3c_c_test` registered: ask the
#   generator for every fixture's plan in one batch, then declare each
#   fixture's generations from the answer the batch kept, and set, in the
#   caller's scope for each, `W3C_C_AOT_TEST_<id>_SOURCES` (every file those
#   generations write) and `W3C_C_AOT_TEST_<id>_REQUIRES` (what their machines
#   need linked).
function(sce_declare_registered_static_w3c_c_tests)
    get_property(_DECLARED GLOBAL PROPERTY _SCE_W3C_C_DECLARED)
    if(_DECLARED)
        message(FATAL_ERROR
            "sce_declare_registered_static_w3c_c_tests() is called twice; every fixture "
            "is registered before the one call that declares them")
    endif()
    set_property(GLOBAL PROPERTY _SCE_W3C_C_DECLARED TRUE)
    get_property(_REGISTERED GLOBAL PROPERTY _SCE_W3C_C_REGISTERED)
    foreach(_ID IN LISTS _REGISTERED)
        get_property(_OUTPUT_DIR GLOBAL PROPERTY "_SCE_W3C_C_${_ID}_OUTPUT_DIR")
        _sce_request_w3c_fixture(${_ID} "${_OUTPUT_DIR}" c11)
    endforeach()
    sce_codegen_plan_prefetch()
    foreach(_ID IN LISTS _REGISTERED)
        get_property(_OUTPUT_DIR GLOBAL PROPERTY "_SCE_W3C_C_${_ID}_OUTPUT_DIR")
        _sce_generate_w3c_fixture(${_ID} "${_OUTPUT_DIR}" c11)
        set(W3C_C_AOT_TEST_${_ID}_SOURCES
            ${_W3C_FIXTURE_SOURCES} ${_W3C_FIXTURE_HEADERS} PARENT_SCOPE)
        set(W3C_C_AOT_TEST_${_ID}_REQUIRES ${_W3C_FIXTURE_REQUIRES} PARENT_SCOPE)
    endforeach()
endfunction()

# _sce_refuse_undeclared_static_w3c_c_tests()
#
#   At the end of the configure: refuse fixtures registered and never
#   declared, whose runners would be compiled against machines nothing
#   generates.
function(_sce_refuse_undeclared_static_w3c_c_tests)
    get_property(_DECLARED GLOBAL PROPERTY _SCE_W3C_C_DECLARED)
    if(NOT _DECLARED)
        get_property(_REGISTERED GLOBAL PROPERTY _SCE_W3C_C_REGISTERED)
        message(FATAL_ERROR
            "sce_generate_static_w3c_c_test registered ${_REGISTERED}, and no "
            "sce_declare_registered_static_w3c_c_tests() declared them")
    endif()
endfunction()
