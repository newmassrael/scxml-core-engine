# SCE Static Integration Fixture Code Generation
#
# Parallel to SCEStaticW3CTest.cmake but scoped to integration
# fixtures under `integration_resources/<stem>/<stem>.scxml` (RFC
# `claudedocs/rfc-donedata-5-backend-layout.md` Q-2). Generates C++
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
# Keyword args:
#   SYNTH_INVOKE_CHILDREN <id>...  IDs of `<invoke id="...">` elements
#                                  whose inline `<content>` is emitted
#                                  as a separate `<stem>__sce_synth_invoke__<id>.scxml`
#                                  state machine by SCE Mesh §9.6.6 rule 1.
#                                  Required when the fixture uses inline
#                                  invoke content (most do); pass the
#                                  ids as they appear in the fixture.
#
# Side effects:
#   - Appends parent + every child `_sm.h` to GENERATED_INTEGRATION_HEADERS
#     (PARENT_SCOPE) so the caller's executable target can DEPENDS on them.
#
# Drift contract:
#   The build-time output dir is distinct from W3C's `w3c_static_generated/`
#   so a stale fixture surfaces as a stale tree per-context (mirroring
#   the committed-tree backends' separate §6.2.6 drift contexts).
function(sce_generate_static_integration_test STEM OUTPUT_DIR)
    cmake_parse_arguments(_INT "" "" "SYNTH_INVOKE_CHILDREN" ${ARGN})

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
    set(PARENT_INL "${OUTPUT_DIR}/${STEM}_sm.inl")

    # Step 1: stage the fixture into OUTPUT_DIR so synth-invoke children
    # land in OUTPUT_DIR rather than polluting the read-only source tree
    # under integration_resources/.
    add_custom_command(
        OUTPUT "${STAGED_SCXML}"
        COMMAND ${CMAKE_COMMAND} -E copy_if_different "${FIXTURE}" "${STAGED_SCXML}"
        DEPENDS "${FIXTURE}"
        COMMENT "Staging integration fixture: ${STEM}.scxml"
        VERBATIM
    )

    # Step 2: parent generate. Emits `<stem>_sm.{h,inl}` plus the
    # synth-invoke `<stem>__sce_synth_invoke__<id>.scxml` siblings.
    # `--input-root` pins the §6.2.6 source-hash to the canonical
    # fixture dir so the build-time output advertises the same hash
    # the committed-tree backends embed.
    set(_CHILD_SCXMLS "")
    foreach(_CHILD ${_INT_SYNTH_INVOKE_CHILDREN})
        list(APPEND _CHILD_SCXMLS "${OUTPUT_DIR}/${STEM}__sce_synth_invoke__${_CHILD}.scxml")
    endforeach()

    if(SCE_CLANG_FORMAT_FOUND)
        set(_PARENT_FMT_CMD
            COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}"
                    -i "${PARENT_HEADER}" "${PARENT_INL}")
    else()
        set(_PARENT_FMT_CMD "")
    endif()

    add_custom_command(
        OUTPUT "${PARENT_HEADER}"
        COMMAND "${SCE_CODEGEN}" generate "${STAGED_SCXML}"
                -l cpp -o "${OUTPUT_DIR}"
                --input-root "${FIXTURE_ROOT}"
        ${_PARENT_FMT_CMD}
        DEPENDS "${STAGED_SCXML}" "${SCE_CODEGEN}"
        BYPRODUCTS "${PARENT_INL}" ${_CHILD_SCXMLS}
        COMMENT "Generating C++ integration parent: ${STEM}_sm.h"
        VERBATIM
    )

    # Step 3: per-child generate. `--as-child --parent-stem` rewrites
    # each child's class namespace so the parent's emitted reference
    # to the child `StateMachine` class resolves in the shared
    # generated tree (mirrors `KotlinBackend::process_child` /
    # `GoBackend::process_child`).
    set(_ALL_HEADERS "${PARENT_HEADER}")
    foreach(_CHILD ${_INT_SYNTH_INVOKE_CHILDREN})
        set(_CHILD_SCXML "${OUTPUT_DIR}/${STEM}__sce_synth_invoke__${_CHILD}.scxml")
        set(_CHILD_HEADER "${OUTPUT_DIR}/${STEM}__sce_synth_invoke__${_CHILD}_sm.h")
        set(_CHILD_INL "${OUTPUT_DIR}/${STEM}__sce_synth_invoke__${_CHILD}_sm.inl")

        if(SCE_CLANG_FORMAT_FOUND)
            set(_CHILD_FMT_CMD
                COMMAND "${SCE_CLANG_FORMAT}" "-style=file:${SCE_CLANG_FORMAT_STYLE}"
                        -i "${_CHILD_HEADER}" "${_CHILD_INL}")
        else()
            set(_CHILD_FMT_CMD "")
        endif()

        add_custom_command(
            OUTPUT "${_CHILD_HEADER}"
            COMMAND "${SCE_CODEGEN}" generate "${_CHILD_SCXML}"
                    --as-child --parent-stem "${STEM}"
                    -l cpp -o "${OUTPUT_DIR}"
                    --input-root "${FIXTURE_ROOT}"
            ${_CHILD_FMT_CMD}
            DEPENDS "${PARENT_HEADER}" "${SCE_CODEGEN}"
            BYPRODUCTS "${_CHILD_INL}"
            COMMENT "Generating C++ integration child: ${STEM}__sce_synth_invoke__${_CHILD}_sm.h"
            VERBATIM
        )
        list(APPEND _ALL_HEADERS "${_CHILD_HEADER}")
    endforeach()

    list(APPEND GENERATED_INTEGRATION_HEADERS ${_ALL_HEADERS})
    set(GENERATED_INTEGRATION_HEADERS ${GENERATED_INTEGRATION_HEADERS} PARENT_SCOPE)
endfunction()
