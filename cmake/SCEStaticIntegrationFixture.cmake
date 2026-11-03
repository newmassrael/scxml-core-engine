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

# Which files a fixture's generation writes is asked of the generator
# (`sce_codegen_plan`), by the same helper the W3C route asks it through.
include(${CMAKE_CURRENT_LIST_DIR}/SCECodegenPlan.cmake)

# _sce_refuse_children_arguments:
#   The synth-invoke children and hybrid stubs of a fixture are derived from the
#   generator (`sce_codegen_plan`), so a call that still lists them
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
#   SCXML_FILE  (optional) The document, when it is NOT under
#               `integration_resources/`. Such a document sits beside its drivers
#               because a stem under `integration_resources/` is a seven-channel
#               contract and the document asks something fewer channels can be
#               asked (see `tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml`).
#               Its file name must be `${STEM}.scxml`, and the directory holding
#               it is the `--input-root` the way the stem's own directory is.
#
# What the fixture produces besides its own `_sm.{h,inl}` — a machine per inline
# `<invoke>` (`<stem>__sce_synth_invoke__<id>`, SCE Mesh §9.6.6 rule 1) and one
# per stub or candidate of an `<invoke>` that names its target through an
# expression (docs/SCE_ACCEPTED_SUBSET.md §2.13) — is not named by the caller.
# It is asked of the generator when the tree is configured
# (`sce_codegen_plan`), and every file of the answer is declared
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
    cmake_parse_arguments(_INT "" "SCXML_FILE" "" ${ARGN})
    _sce_refuse_children_arguments(sce_generate_static_integration_test "${STEM}"
        ${_INT_UNPARSED_ARGUMENTS})

    if(_INT_SCXML_FILE)
        set(FIXTURE "${_INT_SCXML_FILE}")
        get_filename_component(FIXTURE_ROOT "${FIXTURE}" DIRECTORY)
    else()
        set(FIXTURE_ROOT "${CMAKE_SOURCE_DIR}/integration_resources/${STEM}")
        set(FIXTURE "${FIXTURE_ROOT}/${STEM}.scxml")
    endif()

    if(NOT EXISTS "${FIXTURE}")
        message(WARNING
            "sce_generate_static_integration_test: fixture not found, "
            "skipping ${STEM}: ${FIXTURE}")
        return()
    endif()

    file(MAKE_DIRECTORY "${OUTPUT_DIR}")

    set(STAGED_SCXML "${OUTPUT_DIR}/${STEM}.scxml")

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
    sce_codegen_plan(_PLAN
        STEM "${STEM}"
        DOCUMENT "${STAGED_SCXML}"
        OUTPUT_DIR "${OUTPUT_DIR}"
        LANGUAGE cpp
        INPUT_ROOT "${FIXTURE_ROOT}")
    list(GET _PLAN_OUTPUTS 0 PARENT_HEADER)

    # Formatting happens inside sce-codegen, with the pinned clang-format, on
    # the parent and the synth children it emits alike — see SCEClangFormat.cmake.
    sce_codegen_format_args(_FORMAT_ARGS)

    # `--write-deps` + DEPFILE: the generated source depends on every
    # jinja2 template that rendered it, not just on the SCXML. Without it
    # a template edit leaves this artefact stale and the build reuses it
    # silently — measured, by editing a template and watching zero of the
    # generated headers change. `SCECodegen.cmake` has always done this;
    # this generator had not.
    add_custom_command(
        OUTPUT ${_PLAN_OUTPUTS}
        COMMAND "${SCE_CODEGEN}" generate "${STAGED_SCXML}"
                -l cpp -o "${OUTPUT_DIR}"
                --input-root "${FIXTURE_ROOT}"
                --write-deps "${_PLAN_DEPFILE}"
                ${_FORMAT_ARGS}
        DEPENDS "${STAGED_SCXML}" "${SCE_CODEGEN}"
        DEPFILE "${_PLAN_DEPFILE}"
        ${_PLAN_BYPRODUCTS_ARG}
        COMMENT "Generating C++ integration parent: ${STEM}"
        VERBATIM
    )

    # The parent's header and every header its generation writes beside it:
    # each synthesized child's, and the transition function's when the document
    # declares one.
    set(_ALL_HEADERS ${_PLAN_HEADERS})

    # Hybrid children are generated without `--parent-stem`; see above.
    foreach(_HYBRID ${_PLAN_CHILDREN})
        set(_HYBRID_SCXML "${OUTPUT_DIR}/${_HYBRID}.scxml")
        set(_HYBRID_HEADER "${OUTPUT_DIR}/${_HYBRID}_sm.h")
        set(_HYBRID_INL "${OUTPUT_DIR}/${_HYBRID}_sm.inl")

        set(_HYBRID_DEPFILE "${_HYBRID_HEADER}.d")

        # A candidate that starts children of its own written inline (SCE_MESH.md,
        # Child Sessions in Mesh: `static_mesh_relay` and its `inner`) has its
        # generation emit them beside it, so they are declared as its outputs,
        # from the plan of the candidate's own document.
        # A stub the parent's generation wrote is no document of the fixture's
        # and starts nothing.
        set(_HYBRID_PLAN_SYNTH "")
        if(EXISTS "${FIXTURE_ROOT}/${_HYBRID}.scxml")
            sce_codegen_plan(_HYBRID_PLAN
                STEM "${_HYBRID}"
                DOCUMENT "${FIXTURE_ROOT}/${_HYBRID}.scxml"
                OUTPUT_DIR "${OUTPUT_DIR}"
                LANGUAGE cpp
                INPUT_ROOT "${FIXTURE_ROOT}"
                ARGS --as-child)
        endif()
        # One the parent's own generation already declares is its, not this one's.
        if(_HYBRID_PLAN_SYNTH AND _PLAN_SYNTH)
            list(REMOVE_ITEM _HYBRID_PLAN_SYNTH ${_PLAN_SYNTH})
        endif()
        set(_HYBRID_BYPRODUCTS "${_HYBRID_INL}")
        foreach(_CHILD ${_HYBRID_PLAN_SYNTH})
            list(APPEND _HYBRID_BYPRODUCTS "${OUTPUT_DIR}/${_CHILD}_sm.h")
        endforeach()

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
            BYPRODUCTS ${_HYBRID_BYPRODUCTS}
            COMMENT "Generating C++ integration hybrid child: ${_HYBRID}_sm.h"
            VERBATIM
        )
        list(APPEND _ALL_HEADERS "${_HYBRID_HEADER}")
        foreach(_CHILD ${_HYBRID_PLAN_SYNTH})
            list(APPEND _ALL_HEADERS "${OUTPUT_DIR}/${_CHILD}_sm.h")
        endforeach()
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
# variant's are: see `sce_codegen_plan`.
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
#                            A library name (`sce:std/shared/record_list.scxml`)
#                            generates that standard statechart from the
#                            library the generator embeds, unstaged.
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

    # A library name (`sce:std/shared/record_list.scxml`) names a statechart of the
    # standard library, which the generator reads from the library it embeds: it is
    # neither found on disk nor staged, it has no directory to be the input root, and
    # its siblings are the library's, which the generator resolves itself.
    set(_STANDARD FALSE)
    if(_INT_SCXML_FILE MATCHES "^sce:")
        set(_STANDARD TRUE)
        if(_INT_STAGE_ALSO)
            message(FATAL_ERROR
                "sce_generate_static_integration_c_test(${STEM}): STAGE_ALSO names "
                "${_INT_STAGE_ALSO} beside ${_INT_SCXML_FILE}, a standard statechart, "
                "whose imports the generator reads from its own library")
        endif()
        set(FIXTURE_ROOT "")
        set(FIXTURE "${_INT_SCXML_FILE}")
    elseif(_INT_SCXML_FILE)
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

    if(NOT _STANDARD AND NOT EXISTS "${FIXTURE}")
        message(WARNING
            "sce_generate_static_integration_c_test: fixture not found, "
            "skipping ${STEM}: ${FIXTURE}")
        return()
    endif()

    file(MAKE_DIRECTORY "${OUTPUT_DIR}")

    if(_STANDARD)
        # Generated by its library name; a rebuilt generator is a changed library.
        set(STAGED_SCXML "${FIXTURE}")
        set(_STAGED_DEPENDS "")
        set(_INPUT_ROOT_ARGS "")
    else()
        set(STAGED_SCXML "${OUTPUT_DIR}/${STEM}.scxml")
        set(_STAGED_DEPENDS "${STAGED_SCXML}")
        set(_INPUT_ROOT_ARGS --input-root "${FIXTURE_ROOT}")
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
    endif()

    # The parent emits each synth-invoke child's `_sm.{h,c}` with its own —
    # see the cpp generator above for why they are declared here and not
    # by a command of their own, and why the declaration is the generator's
    # answer and not a list.
    sce_codegen_plan(_PLAN
        STEM "${STEM}"
        DOCUMENT "${STAGED_SCXML}"
        OUTPUT_DIR "${OUTPUT_DIR}"
        LANGUAGE c11
        INPUT_ROOT "${FIXTURE_ROOT}"
        ARGS ${_HOST_PROCESSOR_ARGS})
    list(GET _PLAN_OUTPUTS 0 PARENT_HEADER)
    list(GET _PLAN_OUTPUTS 1 PARENT_SOURCE)
    # A document two fixtures start a child from (the candidates a hybrid `<invoke>` chooses
    # among, which the generation of whoever starts them writes beside it) is the same file
    # whoever writes it, so the first fixture to declare it owns it: two commands declaring one
    # file as an output is an error of its own, and the second writes the same bytes.
    get_property(_DECLARED GLOBAL PROPERTY SCE_C_INTEGRATION_DECLARED_OUTPUTS)
    set(_PARENT_BYPRODUCTS "")
    foreach(_FILE ${_PLAN_BYPRODUCTS})
        if(NOT "${_FILE}" IN_LIST _DECLARED)
            list(APPEND _PARENT_BYPRODUCTS "${_FILE}")
            set_property(GLOBAL APPEND PROPERTY SCE_C_INTEGRATION_DECLARED_OUTPUTS "${_FILE}")
        endif()
    endforeach()

    # See the cpp generator above for why DEPFILE is load-bearing: the C11
    # path is where the stale-artefact defect was first hit, and it took a
    # manual `rm` of the generated `.c` to get a template fix compiled.
    #
    # The header comes first in OUTPUT because it is the depfile's one
    # target (`sce_codegen_plan` says why).
    add_custom_command(
        OUTPUT ${_PLAN_OUTPUTS}
        COMMAND "${SCE_CODEGEN}" generate "${STAGED_SCXML}"
                -l c11 -o "${OUTPUT_DIR}"
                ${_INPUT_ROOT_ARGS}
                ${_HOST_PROCESSOR_ARGS}
                --write-deps "${_PLAN_DEPFILE}"
        DEPENDS ${_STAGED_DEPENDS} "${SCE_CODEGEN}"
        DEPFILE "${_PLAN_DEPFILE}"
        BYPRODUCTS ${_PARENT_BYPRODUCTS}
        COMMENT "Generating C11 integration parent: ${STEM}"
        VERBATIM
    )

    # The parent's machine and every translation unit and header its generation
    # writes beside it: each synthesized child's, and the transition function's
    # when the document declares one.
    set(_ALL_SOURCES ${_PLAN_SOURCES})
    set(_ALL_HEADERS ${_PLAN_HEADERS})

    # Hybrid children, without `--parent-stem` for the reason the cpp
    # generator above states: the parent names the child by the plain
    # symbol codegen gives a document of that name.
    foreach(_HYBRID ${_PLAN_CHILDREN})
        set(_HYBRID_SCXML "${OUTPUT_DIR}/${_HYBRID}.scxml")
        set(_HYBRID_HEADER "${OUTPUT_DIR}/${_HYBRID}_sm.h")
        set(_HYBRID_SOURCE "${OUTPUT_DIR}/${_HYBRID}_sm.c")

        set(_HYBRID_DEPFILE "${_HYBRID_SOURCE}.d")

        # A candidate that starts children of its own written inline (SCE_MESH.md,
        # Child Sessions in Mesh: `static_mesh_relay` and its `inner`) has its
        # generation emit them beside it, so they are declared as its outputs and
        # compiled, from the plan of the candidate's own document. A stub the
        # parent's generation wrote is no document of the fixture's and starts
        # nothing.
        set(_HYBRID_PLAN_SYNTH "")
        if(EXISTS "${FIXTURE_ROOT}/${_HYBRID}.scxml")
            sce_codegen_plan(_HYBRID_PLAN
                STEM "${_HYBRID}"
                DOCUMENT "${FIXTURE_ROOT}/${_HYBRID}.scxml"
                OUTPUT_DIR "${OUTPUT_DIR}"
                LANGUAGE c11
                INPUT_ROOT "${FIXTURE_ROOT}"
                ARGS --as-child)
        endif()
        # One the parent's own generation already declares is its, not this one's.
        if(_HYBRID_PLAN_SYNTH AND _PLAN_SYNTH)
            list(REMOVE_ITEM _HYBRID_PLAN_SYNTH ${_PLAN_SYNTH})
        endif()
        set(_HYBRID_BYPRODUCTS "")
        foreach(_CHILD ${_HYBRID_PLAN_SYNTH})
            list(APPEND _HYBRID_BYPRODUCTS "${OUTPUT_DIR}/${_CHILD}_sm.h" "${OUTPUT_DIR}/${_CHILD}_sm.c")
        endforeach()

        # A candidate two fixtures start is generated by the first to declare it (see the
        # parent's byproducts above); the second is compiled with the same files.
        get_property(_DECLARED_CHILDREN GLOBAL PROPERTY SCE_C_INTEGRATION_DECLARED_CHILDREN)
        if(NOT "${_HYBRID_HEADER}" IN_LIST _DECLARED_CHILDREN)
            set_property(GLOBAL APPEND PROPERTY SCE_C_INTEGRATION_DECLARED_CHILDREN "${_HYBRID_HEADER}")
            add_custom_command(
                OUTPUT "${_HYBRID_HEADER}" "${_HYBRID_SOURCE}"
                COMMAND "${SCE_CODEGEN}" generate "${_HYBRID_SCXML}"
                        --as-child
                        -l c11 -o "${OUTPUT_DIR}"
                        ${_INPUT_ROOT_ARGS}
                        --write-deps "${_HYBRID_DEPFILE}"
                DEPENDS "${PARENT_SOURCE}" "${SCE_CODEGEN}"
                DEPFILE "${_HYBRID_DEPFILE}"
                BYPRODUCTS ${_HYBRID_BYPRODUCTS}
                COMMENT "Generating C11 integration hybrid child: ${_HYBRID}_sm.{h,c}"
                VERBATIM
            )
        endif()
        list(APPEND _ALL_SOURCES "${_HYBRID_SOURCE}")
        list(APPEND _ALL_HEADERS "${_HYBRID_HEADER}")
        foreach(_CHILD ${_HYBRID_PLAN_SYNTH})
            list(APPEND _ALL_SOURCES "${OUTPUT_DIR}/${_CHILD}_sm.c")
            list(APPEND _ALL_HEADERS "${OUTPUT_DIR}/${_CHILD}_sm.h")
        endforeach()
    endforeach()

    # Sources include both .c (compilation units) and .h (generated header
    # dependencies). Listing the headers makes them part of the target's
    # input set so Ninja schedules every child codegen before compiling
    # the parent _sm.c that #includes them.
    set("INTEGRATION_C_TEST_${STEM}_SOURCES" ${_ALL_SOURCES} ${_ALL_HEADERS} PARENT_SCOPE)
endfunction()
