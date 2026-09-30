# SCE Static Integration Fixture Code Generation
#
# Parallel to SCEStaticW3CTest.cmake but scoped to integration
# fixtures under `integration_resources/<stem>/<stem>.scxml`. Generates C++
# state machine code (parent + synth-invoke children) at CMake
# build time into ${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/.
#
# Build-time generation (no committed tree) mirrors how cpp / C11
# W3C IRP code is produced — the build itself is the §6.2.6
# freshness invariant, distinct from the committed-tree backends
# (Rust / Kotlin / Go) where drift-verify guards the SCE-GENERATED
# header.

if(POLICY CMP0116)
    cmake_policy(SET CMP0116 NEW)
endif()

include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

# SCE_CODEGEN is already resolved in SCEStaticW3CTest.cmake. Loading
# this file after that one keeps a single source of truth for the
# binary path; this guard catches the misuse where this file is
# included standalone.
if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCEStaticIntegrationFixture.cmake: SCE_CODEGEN unset. Include "
        "SCEStaticW3CTest.cmake first (it resolves the binary).")
endif()

# _sce_plan_integration_fixture:
#   Ask the generator which files generating one fixture produces.
#
#   A build system has to declare a command's outputs before the command has
#   run, and the files a document produces are not all files its author wrote:
#   an inline `<invoke>` becomes a child document with a machine of its own
#   (`<stem>__sce_synth_invoke__<id>`), and an `<invoke>` that names its target
#   through an expression brings a stub or a candidate with it. They used to be
#   listed by hand at every call site. A list kept beside the document is a
#   second copy of what the document says, and it fell behind it: six fixtures
#   were registered with none of the children their documents synthesize, and
#   the depfile the generator writes then named files no rule declared —
#   "depfile mentions ... as an output, but no such output was declared",
#   which ninja raises on the rebuild of a directory that a fresh build never
#   reads back, and which a newer CMake let through. The answer now comes from
#   `sce-codegen --plan generate`, the same generation run in memory with the
#   same arguments, so there is nothing to keep in step.
#
# Args:
#   PREFIX      Names the variables set in the caller's scope:
#                 <PREFIX>_FILES   the file names (not paths) the run produces,
#                                  in the order it produced them — every one
#                                  lands directly in OUTPUT_DIR
#                 <PREFIX>_SYNTH   the document name of each synthesized child,
#                                  `<stem>__sce_synth_invoke__<id>`
#                 <PREFIX>_HYBRID  the document name of each other child
#                                  document the run leaves beside the parent —
#                                  a stub for an `<invoke srcexpr>`, or a
#                                  candidate the document declares — which each
#                                  needs a machine generated from it
#   STEM, OUTPUT_DIR, LANGUAGE, SCXML, INPUT_ROOT
#               What the real run is given. SCXML must exist now: it is the
#               staged copy, made at configure time for this reason.
#   ARGN        The rest of the real run's arguments that change what it emits
#               (`--host-processor` …). Formatting and depfile arguments are
#               left out: they change bytes, never which files.
#
# `sce_sourcemap.json` is left out of the answer. Every fixture writes it, to
# the one name, in the one directory they share, so no command can own it — two
# commands declaring one file as an output is an error of its own.
function(_sce_plan_integration_fixture PREFIX STEM OUTPUT_DIR LANGUAGE SCXML INPUT_ROOT)
    execute_process(
        COMMAND "${SCE_CODEGEN}" --plan generate "${SCXML}"
                -l "${LANGUAGE}" -o "${OUTPUT_DIR}"
                --input-root "${INPUT_ROOT}"
                ${ARGN}
        RESULT_VARIABLE _PLAN_RESULT
        OUTPUT_VARIABLE _PLAN_OUTPUT
        ERROR_VARIABLE _PLAN_ERROR
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(NOT _PLAN_RESULT EQUAL 0)
        message(FATAL_ERROR
            "sce-codegen --plan generate failed for the integration fixture "
            "${STEM} (${LANGUAGE}), so its outputs cannot be declared. "
            "A sce-codegen older than --plan answers with a usage error: "
            "rebuild it from this tree.\n${_PLAN_ERROR}")
    endif()

    set(_FILES "")
    set(_SYNTH "")
    set(_HYBRID "")
    string(REPLACE "\n" ";" _PLANNED "${_PLAN_OUTPUT}")
    foreach(_PATH ${_PLANNED})
        get_filename_component(_NAME "${_PATH}" NAME)
        if("${_NAME}" STREQUAL "sce_sourcemap.json")
            continue()
        endif()
        file(RELATIVE_PATH _RELATIVE "${OUTPUT_DIR}" "${_PATH}")
        if("${_RELATIVE}" MATCHES "/")
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${STEM} (${LANGUAGE}) names ${_PATH}, "
                "which is not directly in ${OUTPUT_DIR}: this function declares the "
                "outputs of a flat output directory")
        endif()
        list(APPEND _FILES "${_NAME}")
        if("${_NAME}" MATCHES "\\.scxml$")
            get_filename_component(_DOCUMENT "${_NAME}" NAME_WE)
            if("${_DOCUMENT}" MATCHES "__sce_synth_invoke__")
                list(APPEND _SYNTH "${_DOCUMENT}")
            else()
                list(APPEND _HYBRID "${_DOCUMENT}")
            endif()
        endif()
    endforeach()

    # What `--plan` answers depends on the generator as much as on the
    # document, so a rebuilt generator reconfigures the tree. It already
    # regenerates every fixture (each command DEPENDS on it); this is the
    # declaration of what those regenerations are allowed to produce.
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${SCE_CODEGEN}")

    set(${PREFIX}_FILES "${_FILES}" PARENT_SCOPE)
    set(${PREFIX}_SYNTH "${_SYNTH}" PARENT_SCOPE)
    set(${PREFIX}_HYBRID "${_HYBRID}" PARENT_SCOPE)
endfunction()

# _sce_refuse_children_arguments:
#   The synth-invoke children and hybrid stubs of a fixture are derived from the
#   generator (`_sce_plan_integration_fixture`), so a call that still lists them
#   is asking for the second copy this file no longer keeps — and would be
#   ignored without a word if it were not refused.
function(_sce_refuse_children_arguments FUNCTION STEM)
    if(ARGN)
        message(FATAL_ERROR
            "${FUNCTION}(${STEM}): unexpected argument(s): ${ARGN}. "
            "SYNTH_INVOKE_CHILDREN and HYBRID_INVOKE_CHILDREN are gone: the children "
            "a fixture produces are read from `sce-codegen --plan generate`, not "
            "listed here. Remove them from the call.")
    endif()
endfunction()

# sce_generate_static_integration_test:
#   Generate C++ AOT code for one integration fixture.
#
# Args:
#   STEM        Fixture stem (`donedata_local_invoke` etc.). The
#               canonical source must live at
#               `${CMAKE_SOURCE_DIR}/integration_resources/${STEM}/${STEM}.scxml`.
#   OUTPUT_DIR  Destination dir for generated `_sm.{h,inl}` files
#               (typically `${STATIC_INTEGRATION_OUTPUT_DIR}`).
#
# What the fixture produces besides its own `_sm.{h,inl}` — a machine per inline
# `<invoke>` (`<stem>__sce_synth_invoke__<id>`, SCE Mesh §9.6.6 rule 1) and one
# per stub or candidate of an `<invoke>` that names its target through an
# expression (docs/SCE_ACCEPTED_SUBSET.md §2.13) — is not named by the caller.
# It is asked of the generator when the tree is configured
# (`_sce_plan_integration_fixture`), and every file of the answer is declared
# as an output of the command that writes it. The hybrid children are generated
# by a command of their own, without `--parent-stem`: the parent emits
# `::SCE::Generated::<name>::<name>`, the plain namespace codegen gives a
# document of that name, so rewriting it under the parent would leave the
# reference unresolved.
#
# Side effects:
#   - Appends parent + every child `_sm.h` to GENERATED_INTEGRATION_HEADERS
#     (PARENT_SCOPE) so the caller's executable target can DEPENDS on them.
#   - Stages the fixture into OUTPUT_DIR when the tree is configured, so the
#     children land in OUTPUT_DIR rather than in the read-only source tree under
#     integration_resources/. Editing the fixture reconfigures the tree, which
#     is when the children are asked for again.
#
# Drift contract:
#   The build-time output dir is distinct from W3C's `w3c_static_generated/`
#   so a stale fixture surfaces as a stale tree per-context (mirroring
#   the committed-tree backends' separate §6.2.6 drift contexts).
function(sce_generate_static_integration_test STEM OUTPUT_DIR)
    cmake_parse_arguments(_INT "" "" "" ${ARGN})
    _sce_refuse_children_arguments(sce_generate_static_integration_test "${STEM}"
        ${_INT_UNPARSED_ARGUMENTS})

    set(FIXTURE_ROOT "${CMAKE_SOURCE_DIR}/integration_resources/${STEM}")
    set(FIXTURE "${FIXTURE_ROOT}/${STEM}.scxml")

    if(NOT EXISTS "${FIXTURE}")
        message(WARNING
            "sce_generate_static_integration_test: fixture not found, "
            "skipping ${STEM}: ${FIXTURE}")
        return()
    endif()

    file(MAKE_DIRECTORY "${OUTPUT_DIR}")

    set(STAGED_SCXML "${OUTPUT_DIR}/${STEM}.scxml")
    set(PARENT_HEADER "${OUTPUT_DIR}/${STEM}_sm.h")

    # Step 1: stage the fixture into OUTPUT_DIR so synth-invoke children
    # land in OUTPUT_DIR rather than polluting the read-only source tree
    # under integration_resources/. At configure time, because the plan below
    # has to read the staged document; `configure_file` also makes an edit to
    # the fixture reconfigure the tree, which is when its children are asked
    # for again.
    configure_file("${FIXTURE}" "${STAGED_SCXML}" COPYONLY)

    # Step 2: parent generate. Emits `<stem>_sm.{h,inl}`, the synth-invoke
    # `<stem>__sce_synth_invoke__<id>.scxml` siblings with each one's
    # `_sm.{h,inl}` — `generate` emits an inline child's machine with its
    # parent, under the parent's stem, so no `--as-child` pass per child is
    # needed — and any stub or candidate document of an `<invoke srcexpr>`.
    # `--input-root` pins the §6.2.6 source-hash to the canonical fixture dir
    # so the build-time output advertises the same hash the committed-tree
    # backends embed.
    #
    # Every file this command writes is declared on it: the parent header as the
    # OUTPUT and the rest as BYPRODUCTS. The declaration is the generator's own
    # answer (`sce-codegen --plan generate`), not a list kept here. A hand-kept
    # list of a fixture's synth and hybrid children fell out of step with the
    # generator for six stems, and each file it missed was one ninja 1.11 refused
    # a rebuild over: "depfile mentions ... as an output, but no such output was
    # declared" (CMake 3.28, 2026-09-30). The depfile names one target now, so a
    # missing declaration no longer stops the build, but it still leaves a file
    # the build does not know it makes.
    #
    # The header is the OUTPUT because the depfile's one target is the primary
    # artefact, and ninja needs that to be the edge's FIRST output.
    _sce_plan_integration_fixture(_PLAN "${STEM}" "${OUTPUT_DIR}" cpp
        "${STAGED_SCXML}" "${FIXTURE_ROOT}")
    if(NOT "${STEM}_sm.h" IN_LIST _PLAN_FILES)
        message(FATAL_ERROR
            "sce-codegen --plan generate for ${STEM} does not name ${STEM}_sm.h, "
            "the file its generate command is declared to produce")
    endif()
    set(_PARENT_BYPRODUCTS "")
    foreach(_FILE ${_PLAN_FILES})
        if(NOT "${_FILE}" STREQUAL "${STEM}_sm.h")
            list(APPEND _PARENT_BYPRODUCTS "${OUTPUT_DIR}/${_FILE}")
        endif()
    endforeach()

    # Formatting happens inside sce-codegen, with the pinned clang-format, on
    # the parent and the synth children it emits alike — see SCEClangFormat.cmake.
    sce_codegen_format_args(_FORMAT_ARGS)

    # `--write-deps` + DEPFILE: the generated source depends on every
    # jinja2 template that rendered it, not just on the SCXML. Without it
    # a template edit leaves this artefact stale and the build reuses it
    # silently — measured, by editing a template and watching zero of the
    # generated headers change. `SCECodegen.cmake` has always done this;
    # this generator had not.
    set(_PARENT_DEPFILE "${PARENT_HEADER}.d")

    add_custom_command(
        OUTPUT "${PARENT_HEADER}"
        COMMAND "${SCE_CODEGEN}" generate "${STAGED_SCXML}"
                -l cpp -o "${OUTPUT_DIR}"
                --input-root "${FIXTURE_ROOT}"
                --write-deps "${_PARENT_DEPFILE}"
                ${_FORMAT_ARGS}
        DEPENDS "${STAGED_SCXML}" "${SCE_CODEGEN}"
        DEPFILE "${_PARENT_DEPFILE}"
        BYPRODUCTS ${_PARENT_BYPRODUCTS}
        COMMENT "Generating C++ integration parent: ${STEM}_sm.h"
        VERBATIM
    )

    set(_ALL_HEADERS "${PARENT_HEADER}")
    foreach(_CHILD ${_PLAN_SYNTH})
        list(APPEND _ALL_HEADERS "${OUTPUT_DIR}/${_CHILD}_sm.h")
    endforeach()

    # Hybrid children are generated without `--parent-stem`; see above.
    foreach(_HYBRID ${_PLAN_HYBRID})
        set(_HYBRID_SCXML "${OUTPUT_DIR}/${_HYBRID}.scxml")
        set(_HYBRID_HEADER "${OUTPUT_DIR}/${_HYBRID}_sm.h")
        set(_HYBRID_INL "${OUTPUT_DIR}/${_HYBRID}_sm.inl")

        set(_HYBRID_DEPFILE "${_HYBRID_HEADER}.d")

        add_custom_command(
            OUTPUT "${_HYBRID_HEADER}"
            COMMAND "${SCE_CODEGEN}" generate "${_HYBRID_SCXML}"
                    --as-child
                    -l cpp -o "${OUTPUT_DIR}"
                    --input-root "${FIXTURE_ROOT}"
                    --write-deps "${_HYBRID_DEPFILE}"
                    ${_FORMAT_ARGS}
            DEPENDS "${PARENT_HEADER}" "${SCE_CODEGEN}"
            DEPFILE "${_HYBRID_DEPFILE}"
            BYPRODUCTS "${_HYBRID_INL}"
            COMMENT "Generating C++ integration hybrid child: ${_HYBRID}_sm.h"
            VERBATIM
        )
        list(APPEND _ALL_HEADERS "${_HYBRID_HEADER}")
    endforeach()

    list(APPEND GENERATED_INTEGRATION_HEADERS ${_ALL_HEADERS})
    set(GENERATED_INTEGRATION_HEADERS ${GENERATED_INTEGRATION_HEADERS} PARENT_SCOPE)
endfunction()

# sce_generate_static_integration_c_test:
#   C11 sibling of `sce_generate_static_integration_test`. Emits
#   `<stem>_sm.{h,c}` (rather than `.h/.inl` for cpp) plus the
#   synth-invoke `_sm.c` children that the parent's translation unit
#   #includes. Caller wires the resulting source list into an
#   `add_executable` target via the conventional integration runner
#   under `backends/c/tests/integration/test_<stem>.c`.
#
# Args:
#   STEM        Fixture stem (`donedata_local_invoke` etc.). Canonical
#               source: `${CMAKE_SOURCE_DIR}/integration_resources/${STEM}/${STEM}.scxml`.
#   OUTPUT_DIR  Destination dir for `_sm.{h,c}` (e.g.
#               `${SCE_C_INTEGRATION_GENERATED_DIR}`).
#
# The fixture's synth-invoke and hybrid children are derived, as the cpp
# variant's are: see `_sce_plan_integration_fixture`.
#
# Side effects:
#   - Sets `INTEGRATION_C_TEST_${STEM}_SOURCES` (PARENT_SCOPE) to the
#     parent _sm.c plus every child _sm.c — caller links these into
#     the per-fixture executable target.
# Keyword args beyond the stem contract:
#   SCXML_FILE <path>        Generate this document instead of the
#                            `integration_resources/<stem>/<stem>.scxml`
#                            the stem names. For a fixture that is
#                            deliberately NOT a seven-channel stem — the
#                            §scxml-6.2.5 host-served document is asked of
#                            the backends that have a registry, and the
#                            others still refuse the declaration by name.
#   HOST_PROCESSOR <type>... Pass `--host-processor <type>` to codegen.
#                            Load-bearing rather than cosmetic: codegen
#                            decides at compile time whether a `<send
#                            type>` site dispatches or refuses, so a test
#                            registering a handler against a machine built
#                            without the flag meets the refusal and reads
#                            as a feature that does not work.
#   HOST_INVOKER <type>...   Pass `--host-invoker <type>` to codegen — the
#                            `<invoke type>` twin of HOST_PROCESSOR, and
#                            load-bearing for the same reason: codegen
#                            decides at compile time whether the invoke
#                            starts or refuses.
#   STAGE_ALSO <file>...     Sibling documents to copy beside the staged
#                            fixture, named relative to its directory.
#                            The staging copies ONE file, so a fixture
#                            that `<sce:import src="…">`s a sibling
#                            resolves it against a directory the sibling
#                            is not in — and the import is not an error,
#                            so what reaches codegen is a document whose
#                            schema silently went missing. Naming them
#                            here rather than scanning the fixture keeps
#                            the answer explicit: a name that is wrong
#                            fails at the copy, where it can be read.
function(sce_generate_static_integration_c_test STEM OUTPUT_DIR)
    cmake_parse_arguments(_INT "" "SCXML_FILE"
        "HOST_PROCESSOR;HOST_INVOKER;STAGE_ALSO" ${ARGN})
    _sce_refuse_children_arguments(sce_generate_static_integration_c_test "${STEM}"
        ${_INT_UNPARSED_ARGUMENTS})

    if(_INT_SCXML_FILE)
        get_filename_component(FIXTURE_ROOT "${_INT_SCXML_FILE}" DIRECTORY)
        set(FIXTURE "${_INT_SCXML_FILE}")
    else()
        set(FIXTURE_ROOT "${CMAKE_SOURCE_DIR}/integration_resources/${STEM}")
        set(FIXTURE "${FIXTURE_ROOT}/${STEM}.scxml")
    endif()

    set(_HOST_PROCESSOR_ARGS "")
    foreach(_TYPE ${_INT_HOST_PROCESSOR})
        list(APPEND _HOST_PROCESSOR_ARGS --host-processor "${_TYPE}")
    endforeach()
    foreach(_TYPE ${_INT_HOST_INVOKER})
        list(APPEND _HOST_PROCESSOR_ARGS --host-invoker "${_TYPE}")
    endforeach()

    if(NOT EXISTS "${FIXTURE}")
        message(WARNING
            "sce_generate_static_integration_c_test: fixture not found, "
            "skipping ${STEM}: ${FIXTURE}")
        return()
    endif()

    file(MAKE_DIRECTORY "${OUTPUT_DIR}")

    set(STAGED_SCXML "${OUTPUT_DIR}/${STEM}.scxml")
    set(PARENT_HEADER "${OUTPUT_DIR}/${STEM}_sm.h")
    set(PARENT_SOURCE "${OUTPUT_DIR}/${STEM}_sm.c")

    # Staged at configure time, as the cpp variant stages it: the plan below
    # reads the staged document, and `configure_file` makes an edit to the
    # fixture or to a sibling reconfigure the tree.
    foreach(_SIBLING ${_INT_STAGE_ALSO})
        if(NOT EXISTS "${FIXTURE_ROOT}/${_SIBLING}")
            message(FATAL_ERROR
                "sce_generate_static_integration_c_test(${STEM}): STAGE_ALSO names "
                "${_SIBLING}, which is not beside the fixture at ${FIXTURE_ROOT}")
        endif()
        configure_file("${FIXTURE_ROOT}/${_SIBLING}" "${OUTPUT_DIR}/${_SIBLING}" COPYONLY)
    endforeach()
    configure_file("${FIXTURE}" "${STAGED_SCXML}" COPYONLY)

    # The parent emits each synth-invoke child's `_sm.{h,c}` with its own —
    # see the cpp generator above for why they are declared here and not
    # by a command of their own, and why the declaration is the generator's
    # answer and not a list.
    _sce_plan_integration_fixture(_PLAN "${STEM}" "${OUTPUT_DIR}" c11
        "${STAGED_SCXML}" "${FIXTURE_ROOT}" ${_HOST_PROCESSOR_ARGS})
    foreach(_NAMED "${STEM}_sm.h" "${STEM}_sm.c")
        if(NOT "${_NAMED}" IN_LIST _PLAN_FILES)
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${STEM} does not name ${_NAMED}, "
                "a file its generate command is declared to produce")
        endif()
    endforeach()
    set(_PARENT_BYPRODUCTS "")
    foreach(_FILE ${_PLAN_FILES})
        if(NOT "${_FILE}" STREQUAL "${STEM}_sm.h"
           AND NOT "${_FILE}" STREQUAL "${STEM}_sm.c")
            list(APPEND _PARENT_BYPRODUCTS "${OUTPUT_DIR}/${_FILE}")
        endif()
    endforeach()

    # See the cpp generator above for why DEPFILE is load-bearing: the C11
    # path is where the stale-artefact defect was first hit, and it took a
    # manual `rm` of the generated `.c` to get a template fix compiled.
    #
    # The header comes first in OUTPUT because it is the depfile's one
    # target. With the source first, ninja on CMake 3.x read the depfile as
    # describing another edge ("expected depfile ... to mention '_sm.c', got
    # '_sm.h'") and regenerated every C11 parent on every build.
    set(_PARENT_DEPFILE "${PARENT_SOURCE}.d")

    add_custom_command(
        OUTPUT "${PARENT_HEADER}" "${PARENT_SOURCE}"
        COMMAND "${SCE_CODEGEN}" generate "${STAGED_SCXML}"
                -l c11 -o "${OUTPUT_DIR}"
                --input-root "${FIXTURE_ROOT}"
                ${_HOST_PROCESSOR_ARGS}
                --write-deps "${_PARENT_DEPFILE}"
        DEPENDS "${STAGED_SCXML}" "${SCE_CODEGEN}"
        DEPFILE "${_PARENT_DEPFILE}"
        BYPRODUCTS ${_PARENT_BYPRODUCTS}
        COMMENT "Generating C11 integration parent: ${STEM}_sm.c"
        VERBATIM
    )

    set(_ALL_SOURCES "${PARENT_SOURCE}")
    set(_ALL_HEADERS "${PARENT_HEADER}")
    foreach(_CHILD ${_PLAN_SYNTH})
        list(APPEND _ALL_SOURCES "${OUTPUT_DIR}/${_CHILD}_sm.c")
        list(APPEND _ALL_HEADERS "${OUTPUT_DIR}/${_CHILD}_sm.h")
    endforeach()

    # Hybrid children, without `--parent-stem` for the reason the cpp
    # generator above states: the parent names the child by the plain
    # symbol codegen gives a document of that name.
    foreach(_HYBRID ${_PLAN_HYBRID})
        set(_HYBRID_SCXML "${OUTPUT_DIR}/${_HYBRID}.scxml")
        set(_HYBRID_HEADER "${OUTPUT_DIR}/${_HYBRID}_sm.h")
        set(_HYBRID_SOURCE "${OUTPUT_DIR}/${_HYBRID}_sm.c")

        set(_HYBRID_DEPFILE "${_HYBRID_SOURCE}.d")

        add_custom_command(
            OUTPUT "${_HYBRID_HEADER}" "${_HYBRID_SOURCE}"
            COMMAND "${SCE_CODEGEN}" generate "${_HYBRID_SCXML}"
                    --as-child
                    -l c11 -o "${OUTPUT_DIR}"
                    --input-root "${FIXTURE_ROOT}"
                    --write-deps "${_HYBRID_DEPFILE}"
            DEPENDS "${PARENT_SOURCE}" "${SCE_CODEGEN}"
            DEPFILE "${_HYBRID_DEPFILE}"
            COMMENT "Generating C11 integration hybrid child: ${_HYBRID}_sm.{h,c}"
            VERBATIM
        )
        list(APPEND _ALL_SOURCES "${_HYBRID_SOURCE}")
        list(APPEND _ALL_HEADERS "${_HYBRID_HEADER}")
    endforeach()

    # Sources include both .c (compilation units) and .h (generated header
    # dependencies). Listing the headers makes them part of the target's
    # input set so Ninja schedules every child codegen before compiling
    # the parent _sm.c that #includes them.
    set("INTEGRATION_C_TEST_${STEM}_SOURCES" ${_ALL_SOURCES} ${_ALL_HEADERS} PARENT_SCOPE)
endfunction()
