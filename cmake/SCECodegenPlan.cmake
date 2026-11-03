# SCE Codegen Plan
#
# Which files one `sce-codegen generate` run writes, asked of the generator.
#
# A build system has to declare a command's outputs before the command has
# run, and the files a document produces are not all files its author wrote:
# an inline `<invoke>` becomes a child document with a machine of its own
# (`<stem>__sce_synth_invoke__<id>`), an `<invoke>` that names its target
# through an expression brings a stub or a candidate with it, and a document
# that declares its transition function has a header of it written beside the
# machine. Both CMake routes that generate at build time used to list those
# files by hand at every call site, and a list kept beside the document is a
# second copy of what the document says that falls behind it. The integration
# route registered six fixtures with none of the children their documents
# synthesize, and the depfile the generator writes then named files no rule
# declared: "depfile mentions ... as an output, but no such output was
# declared", which ninja raises on the rebuild of a directory that a fresh
# build never reads back. The conformance route could not build a document
# declaring its transition function at all, and a test held the corpus to
# never declaring one.
#
# The answer comes from `sce-codegen --plan generate`, the same generation run
# in memory with the same arguments, so there is nothing to keep in step. This
# module is the one place that asks it: `SCEStaticIntegrationFixture.cmake`
# and `SCEStaticW3CTest.cmake` both declare what `sce_codegen_plan` answers,
# and `the_conformance_cmake_route_declares_what_the_generator_plans` holds
# both to that.

include_guard(GLOBAL)

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCECodegenPlan.cmake: SCE_CODEGEN unset. Include SCEStaticW3CTest.cmake "
        "first (it resolves the binary through SCEFindCodegen.cmake).")
endif()

# sce_codegen_plan(<PREFIX>
#     STEM <stem> DOCUMENT <document> OUTPUT_DIR <dir> LANGUAGE <cpp|c11>
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [ARGS <arg>...])
#
#   Ask the generator which files generating DOCUMENT into OUTPUT_DIR writes.
#
#   STEM        The name the run gives the machine: the document's own stem,
#               which names its files.
#   DOCUMENT    A document on disk now, or the library name of a standard
#               statechart (`sce:std/...`), which the generator reads from the
#               library it embeds.
#   OUTPUT_DIR  Where the run writes. Every file of the answer has to land
#               directly in it: this declares the outputs of a flat directory.
#   LANGUAGE    `cpp` or `c11`, the two languages generated at build time.
#   INPUT_ROOT  The `--input-root` the run is given; empty passes none, as the
#               run of a library name passes none.
#   CHILDREN_HELD_IN
#               A directory that holds, beside the document, the child
#               documents it starts. A child document the run names that this
#               directory holds is staged from it into OUTPUT_DIR now, at
#               configure time, and is an input of the run and of the child's
#               own generation rather than an output: the run reads it to
#               learn what the child sends its parent, and a file read before
#               the build starts cannot be one the build makes. Without it,
#               every child document the run names is one of its outputs.
#   ARGS        The rest of the run's arguments that change what it emits
#               (`--as-child`, `--host-processor` ...). Formatting and depfile
#               arguments are left out: they change bytes, never which files.
#
#   Sets, in the caller's scope:
#     <PREFIX>_FILES          the names (not paths) of the files the run
#                             writes, in the order it first wrote them
#     <PREFIX>_SYNTH          the document name of each synthesized child
#     <PREFIX>_CHILDREN       the document name of each other child document
#                             the run names, each of which needs a machine of
#                             its own generated from it
#     <PREFIX>_STAGED         the path of each child document staged from
#                             CHILDREN_HELD_IN
#     <PREFIX>_OUTPUTS        the paths of the machine, header first: the
#                             command's OUTPUT, and the header is the
#                             depfile's one target, which ninja needs to be
#                             the edge's first output
#     <PREFIX>_BYPRODUCTS     the paths of every other file of <PREFIX>_FILES
#     <PREFIX>_BYPRODUCTS_ARG `BYPRODUCTS` followed by them, or nothing when
#                             there are none
#     <PREFIX>_DEPFILE        the depfile the run is asked to write with
#                             `--write-deps`, and the command's DEPFILE
#     <PREFIX>_HEADERS        the paths of every header the run writes
#     <PREFIX>_SOURCES        the paths of every C translation unit it writes
#
# `sce_sourcemap.json` is left out of the answer. Every document generated into
# one directory writes it, to the one name, so no command can own it: two
# commands declaring one file as an output is an error of its own.
function(sce_codegen_plan PREFIX)
    cmake_parse_arguments(PARSE_ARGV 1 _SCP ""
        "STEM;DOCUMENT;OUTPUT_DIR;LANGUAGE;INPUT_ROOT;CHILDREN_HELD_IN" "ARGS")
    if(_SCP_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_plan(${PREFIX}): unexpected argument(s): ${_SCP_UNPARSED_ARGUMENTS}")
    endif()
    foreach(_REQUIRED STEM DOCUMENT OUTPUT_DIR LANGUAGE)
        if("${_SCP_${_REQUIRED}}" STREQUAL "")
            message(FATAL_ERROR "sce_codegen_plan(${PREFIX}): ${_REQUIRED} is required")
        endif()
    endforeach()
    set(_STEM "${_SCP_STEM}")
    set(_OUTPUT_DIR "${_SCP_OUTPUT_DIR}")

    # The machine a run of each language writes for its document, and the file
    # its depfile is written beside. The one place a build names them: every
    # declaration of a machine reads them from here, and the plan below is held
    # to naming them.
    if(_SCP_LANGUAGE STREQUAL "cpp")
        set(_MACHINE "${_STEM}_sm.h")
        set(_DEPFILE "${_OUTPUT_DIR}/${_STEM}_sm.h.d")
    elseif(_SCP_LANGUAGE STREQUAL "c11")
        # The source is the second output and the depfile sits beside it. With
        # the source first, ninja on CMake 3.x read the depfile as describing
        # another edge ("expected depfile ... to mention '_sm.c', got '_sm.h'")
        # and regenerated every C11 machine on every build.
        set(_MACHINE "${_STEM}_sm.h" "${_STEM}_sm.c")
        set(_DEPFILE "${_OUTPUT_DIR}/${_STEM}_sm.c.d")
    else()
        message(FATAL_ERROR
            "sce_codegen_plan(${PREFIX}): LANGUAGE ${_SCP_LANGUAGE} is generated at "
            "build time by no CMake route; the routes generate cpp and c11")
    endif()

    set(_INPUT_ROOT_ARGS "")
    if(_SCP_INPUT_ROOT)
        set(_INPUT_ROOT_ARGS --input-root "${_SCP_INPUT_ROOT}")
    endif()
    execute_process(
        COMMAND "${SCE_CODEGEN}" --plan generate "${_SCP_DOCUMENT}"
                -l "${_SCP_LANGUAGE}" -o "${_OUTPUT_DIR}"
                ${_INPUT_ROOT_ARGS}
                ${_SCP_ARGS}
        RESULT_VARIABLE _PLAN_RESULT
        OUTPUT_VARIABLE _PLAN_OUTPUT
        ERROR_VARIABLE _PLAN_ERROR
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(NOT _PLAN_RESULT EQUAL 0)
        message(FATAL_ERROR
            "sce-codegen --plan generate failed for ${_SCP_DOCUMENT} (${_SCP_LANGUAGE}), "
            "so the outputs of its generation cannot be declared. A sce-codegen older "
            "than --plan answers with a usage error: rebuild it from this tree.\n"
            "${_PLAN_ERROR}")
    endif()

    set(_FILES "")
    set(_SYNTH "")
    set(_CHILDREN "")
    set(_STAGED "")
    string(REPLACE "\n" ";" _PLANNED "${_PLAN_OUTPUT}")
    foreach(_PATH ${_PLANNED})
        get_filename_component(_NAME "${_PATH}" NAME)
        if("${_NAME}" STREQUAL "sce_sourcemap.json")
            continue()
        endif()
        file(RELATIVE_PATH _RELATIVE "${_OUTPUT_DIR}" "${_PATH}")
        if("${_RELATIVE}" MATCHES "/")
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${_SCP_DOCUMENT} (${_SCP_LANGUAGE}) names "
                "${_PATH}, which is not directly in ${_OUTPUT_DIR}: sce_codegen_plan "
                "declares the outputs of a flat output directory")
        endif()
        if("${_NAME}" MATCHES "\\.scxml$")
            string(REGEX REPLACE "\\.scxml$" "" _CHILD "${_NAME}")
            if("${_CHILD}" MATCHES "__sce_synth_invoke__")
                list(APPEND _SYNTH "${_CHILD}")
            else()
                list(APPEND _CHILDREN "${_CHILD}")
                if(_SCP_CHILDREN_HELD_IN AND EXISTS "${_SCP_CHILDREN_HELD_IN}/${_NAME}")
                    # `configure_file` also makes an edit to the child reconfigure
                    # the tree, which is when the plan is asked again.
                    configure_file("${_SCP_CHILDREN_HELD_IN}/${_NAME}"
                        "${_OUTPUT_DIR}/${_NAME}" COPYONLY)
                    list(APPEND _STAGED "${_OUTPUT_DIR}/${_NAME}")
                    continue()
                endif()
            endif()
        endif()
        list(APPEND _FILES "${_NAME}")
    endforeach()

    foreach(_NAMED ${_MACHINE})
        if(NOT "${_NAMED}" IN_LIST _FILES)
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${_SCP_DOCUMENT} (${_SCP_LANGUAGE}) does "
                "not name ${_NAMED}, a file its generate command is declared to produce")
        endif()
    endforeach()

    set(_OUTPUTS "")
    foreach(_NAMED ${_MACHINE})
        list(APPEND _OUTPUTS "${_OUTPUT_DIR}/${_NAMED}")
    endforeach()
    set(_BYPRODUCTS "")
    set(_HEADERS "")
    set(_SOURCES "")
    foreach(_NAME ${_FILES})
        if(NOT "${_NAME}" IN_LIST _MACHINE)
            list(APPEND _BYPRODUCTS "${_OUTPUT_DIR}/${_NAME}")
        endif()
        if("${_NAME}" MATCHES "\\.h$")
            list(APPEND _HEADERS "${_OUTPUT_DIR}/${_NAME}")
        elseif("${_NAME}" MATCHES "\\.c$")
            list(APPEND _SOURCES "${_OUTPUT_DIR}/${_NAME}")
        endif()
    endforeach()
    # The keyword only when there is something to name: most documents start no
    # child and declare nothing beside their machine.
    set(_BYPRODUCTS_ARG "")
    if(_BYPRODUCTS)
        set(_BYPRODUCTS_ARG BYPRODUCTS ${_BYPRODUCTS})
    endif()

    # What `--plan` answers depends on the generator as much as on the
    # document, so a rebuilt generator reconfigures the tree. It already
    # regenerates every document (each command DEPENDS on it); this is the
    # declaration of what those regenerations are allowed to produce.
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${SCE_CODEGEN}")

    set(${PREFIX}_FILES "${_FILES}" PARENT_SCOPE)
    set(${PREFIX}_SYNTH "${_SYNTH}" PARENT_SCOPE)
    set(${PREFIX}_CHILDREN "${_CHILDREN}" PARENT_SCOPE)
    set(${PREFIX}_STAGED "${_STAGED}" PARENT_SCOPE)
    set(${PREFIX}_OUTPUTS "${_OUTPUTS}" PARENT_SCOPE)
    set(${PREFIX}_BYPRODUCTS "${_BYPRODUCTS}" PARENT_SCOPE)
    set(${PREFIX}_BYPRODUCTS_ARG "${_BYPRODUCTS_ARG}" PARENT_SCOPE)
    set(${PREFIX}_DEPFILE "${_DEPFILE}" PARENT_SCOPE)
    set(${PREFIX}_HEADERS "${_HEADERS}" PARENT_SCOPE)
    set(${PREFIX}_SOURCES "${_SOURCES}" PARENT_SCOPE)
endfunction()
