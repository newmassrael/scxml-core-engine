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
#
# A child document a generation writes into its output directory for a
# generation of its own (a hybrid `<invoke>`'s stub, a candidate the run
# stages) is not on disk when the tree is configured, so a plan of it cannot be
# asked by naming it. `CHILD_GENERATIONS` asks the generator for those plans
# with the run's own (`--plan-children`): the generator plans each child from
# the text the run would write, and `sce_codegen_plan(<PREFIX> FROM <plan>
# CHILD <name>)` declares one of them as a plan of its own.

include_guard(GLOBAL)

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCECodegenPlan.cmake: SCE_CODEGEN unset. Include SCEStaticW3CTest.cmake "
        "first (it resolves the binary through SCEFindCodegen.cmake).")
endif()

# _sce_codegen_plan_machine(<MACHINE_VAR> <DEPFILE_VAR> <STEM> <LANGUAGE>)
#
#   The machine a generation of each language writes for the document of
#   STEM, header first, and the name of the depfile it is asked to write
#   beside it. The one place a build names them: every declaration of a
#   machine reads them from here, and every plan is held to naming them.
function(_sce_codegen_plan_machine MACHINE_VAR DEPFILE_VAR STEM LANGUAGE)
    if(LANGUAGE STREQUAL "cpp")
        set(${MACHINE_VAR} "${STEM}_sm.h" PARENT_SCOPE)
        set(${DEPFILE_VAR} "${STEM}_sm.h.d" PARENT_SCOPE)
    elseif(LANGUAGE STREQUAL "c11")
        # The source is the second output and the depfile sits beside it. With
        # the source first, ninja on CMake 3.x read the depfile as describing
        # another edge ("expected depfile ... to mention '_sm.c', got '_sm.h'")
        # and regenerated every C11 machine on every build.
        set(${MACHINE_VAR} "${STEM}_sm.h" "${STEM}_sm.c" PARENT_SCOPE)
        set(${DEPFILE_VAR} "${STEM}_sm.c.d" PARENT_SCOPE)
    else()
        message(FATAL_ERROR
            "sce_codegen_plan: LANGUAGE ${LANGUAGE} is generated at build time by no "
            "CMake route; the routes generate cpp and c11")
    endif()
endfunction()

# sce_codegen_plan(<PREFIX>
#     STEM <stem> DOCUMENT <document> OUTPUT_DIR <dir> LANGUAGE <cpp|c11>
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [CHILD_GENERATIONS]
#     [CLAIM <global property>]
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
#   CHILD_GENERATIONS
#               Also plan the generation of every child document the run
#               writes, and of every child document those write in turn
#               (`sce-codegen --plan-children`): `generate <child> --as-child`
#               with the run's language, output directory and input root. Each
#               is declared by `sce_codegen_plan(<P> FROM <PREFIX> CHILD
#               <name>)`; a file two generations would write is the first's.
#   CLAIM       A global property naming every file a plan of this property
#               has declared. A file already in it is left out of this plan's
#               byproducts, and a plan whose machine is already in it is one
#               another command declares (`<PREFIX>_DECLARED_ELSEWHERE`):
#               two commands declaring one file is an error of its own, and a
#               document two plans write is the same file whoever writes it.
#               The generations a CHILD_GENERATIONS plan holds claim in it too.
#   ARGS        The rest of the run's arguments that change what it emits
#               (`--as-child`, `--host-processor` ...). Formatting and depfile
#               arguments are left out: they change bytes, never which files.
#
# sce_codegen_plan(<PREFIX> FROM <plan> CHILD <name>)
#
#   Declare the generation of the child document <name> that the plan <plan>,
#   asked with CHILD_GENERATIONS, holds: the same answer, read from the
#   generator once, as a plan of its own.
#
#   Sets, in the caller's scope, besides the variables below:
#     <PREFIX>_DOCUMENT       the child document, which the generation reads
#     <PREFIX>_DEPENDS        that document and the machine of the generation
#                             that writes it, which the command depends on
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
#     <PREFIX>_GENERATIONS    with CHILD_GENERATIONS, the name of each child
#                             document whose generation the plan holds, in the
#                             order the generator planned them (a document
#                             before the documents its generation writes)
#     <PREFIX>_OUTPUTS        the paths of the machine, header first: the
#                             command's OUTPUT, and the header is the
#                             depfile's one target, which ninja needs to be
#                             the edge's first output
#     <PREFIX>_BYPRODUCTS     the paths of every other file of <PREFIX>_FILES
#                             no plan of CLAIM declared before
#     <PREFIX>_BYPRODUCTS_ARG `BYPRODUCTS` followed by them, or nothing when
#                             there are none
#     <PREFIX>_DEPFILE        the depfile the run is asked to write with
#                             `--write-deps`, and the command's DEPFILE
#     <PREFIX>_HEADERS        the paths of every header the run writes
#     <PREFIX>_SOURCES        the paths of every C translation unit it writes
#     <PREFIX>_DECLARED_ELSEWHERE
#                             TRUE when a plan of CLAIM declared this machine
#                             before, so its command is that plan's
#
# `sce_sourcemap.json` is left out of the answer. Every document generated into
# one directory writes it, to the one name, so no command can own it: two
# commands declaring one file as an output is an error of its own.
function(sce_codegen_plan PREFIX)
    cmake_parse_arguments(PARSE_ARGV 1 _SCP "CHILD_GENERATIONS"
        "STEM;DOCUMENT;OUTPUT_DIR;LANGUAGE;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM;FROM;CHILD"
        "ARGS")
    if(_SCP_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_plan(${PREFIX}): unexpected argument(s): ${_SCP_UNPARSED_ARGUMENTS}")
    endif()

    set(_FILES "")
    set(_SYNTH "")
    set(_CHILDREN "")
    set(_STAGED "")
    set(_GENERATIONS "")
    set(_DOCUMENT "")
    set(_DEPENDS "")

    if(_SCP_FROM)
        # ── A generation another plan holds ─────────────────────────────
        foreach(_GIVEN STEM DOCUMENT OUTPUT_DIR LANGUAGE INPUT_ROOT CHILDREN_HELD_IN CLAIM ARGS)
            if(NOT "${_SCP_${_GIVEN}}" STREQUAL "")
                message(FATAL_ERROR
                    "sce_codegen_plan(${PREFIX} FROM ${_SCP_FROM}): ${_GIVEN} is the plan's "
                    "to state, not this call's")
            endif()
        endforeach()
        if(_SCP_CHILD_GENERATIONS)
            message(FATAL_ERROR
                "sce_codegen_plan(${PREFIX} FROM ${_SCP_FROM}): CHILD_GENERATIONS is the "
                "plan's to state; its generations are its own to declare")
        endif()
        if("${_SCP_CHILD}" STREQUAL "")
            message(FATAL_ERROR "sce_codegen_plan(${PREFIX} FROM ${_SCP_FROM}): CHILD is required")
        endif()
        if(NOT "${_SCP_CHILD}" IN_LIST ${_SCP_FROM}_GENERATIONS)
            message(FATAL_ERROR
                "sce_codegen_plan(${PREFIX} FROM ${_SCP_FROM} CHILD ${_SCP_CHILD}): the plan "
                "holds no generation of ${_SCP_CHILD} (it holds: "
                "${${_SCP_FROM}_GENERATIONS}). Ask it with CHILD_GENERATIONS.")
        endif()
        set(_FROM "${_SCP_FROM}_GENERATION_${_SCP_CHILD}")
        set(_STEM "${_SCP_CHILD}")
        set(_OUTPUT_DIR "${${_SCP_FROM}_OUTPUT_DIR}")
        set(_LANGUAGE "${${_SCP_FROM}_LANGUAGE}")
        set(_CLAIM "${${_SCP_FROM}_CLAIM}")
        set(_DOCUMENT "${${_FROM}_DOCUMENT}")
        set(_DEPENDS "${_DOCUMENT}")
        if(NOT "${${_FROM}_WRITER}" STREQUAL "")
            list(APPEND _DEPENDS "${${_FROM}_WRITER}")
        endif()
        foreach(_NAME ${${_FROM}_FILES})
            if("${_NAME}" MATCHES "\\.scxml$")
                string(REGEX REPLACE "\\.scxml$" "" _CHILD "${_NAME}")
                if("${_CHILD}" MATCHES "__sce_synth_invoke__")
                    list(APPEND _SYNTH "${_CHILD}")
                else()
                    list(APPEND _CHILDREN "${_CHILD}")
                endif()
            endif()
            list(APPEND _FILES "${_NAME}")
        endforeach()
    else()
        # ── The generator's answer for one run ──────────────────────────
        if(_SCP_CHILD)
            message(FATAL_ERROR
                "sce_codegen_plan(${PREFIX}): CHILD names a generation of the plan FROM names")
        endif()
        foreach(_REQUIRED STEM DOCUMENT OUTPUT_DIR LANGUAGE)
            if("${_SCP_${_REQUIRED}}" STREQUAL "")
                message(FATAL_ERROR "sce_codegen_plan(${PREFIX}): ${_REQUIRED} is required")
            endif()
        endforeach()
        set(_STEM "${_SCP_STEM}")
        set(_OUTPUT_DIR "${_SCP_OUTPUT_DIR}")
        set(_LANGUAGE "${_SCP_LANGUAGE}")
        set(_CLAIM "${_SCP_CLAIM}")
        # The language is judged before the generator is asked.
        _sce_codegen_plan_machine(_MACHINE _DEPFILE_NAME "${_STEM}" "${_LANGUAGE}")

        set(_INPUT_ROOT_ARGS "")
        if(_SCP_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_SCP_INPUT_ROOT}")
        endif()
        set(_CHILD_GENERATIONS_ARGS "")
        if(_SCP_CHILD_GENERATIONS)
            set(_CHILD_GENERATIONS_ARGS --plan-children)
        endif()
        execute_process(
            COMMAND "${SCE_CODEGEN}" --plan ${_CHILD_GENERATIONS_ARGS}
                    generate "${_SCP_DOCUMENT}"
                    -l "${_LANGUAGE}" -o "${_OUTPUT_DIR}"
                    ${_INPUT_ROOT_ARGS}
                    ${_SCP_ARGS}
            RESULT_VARIABLE _PLAN_RESULT
            OUTPUT_VARIABLE _PLAN_OUTPUT
            ERROR_VARIABLE _PLAN_ERROR
            OUTPUT_STRIP_TRAILING_WHITESPACE
        )
        if(NOT _PLAN_RESULT EQUAL 0)
            message(FATAL_ERROR
                "sce-codegen --plan generate failed for ${_SCP_DOCUMENT} (${_LANGUAGE}), "
                "so the outputs of its generation cannot be declared. A sce-codegen older "
                "than --plan (or, asked with CHILD_GENERATIONS, than --plan-children) "
                "answers with a usage error: rebuild it from this tree.\n"
                "${_PLAN_ERROR}")
        endif()

        # A line with a tab is a file of a child document's generation,
        # `<child document>\t<file>`; every other line is the run's own.
        set(_GENERATION_LINES "")
        string(REPLACE "\n" ";" _PLANNED "${_PLAN_OUTPUT}")
        foreach(_LINE ${_PLANNED})
            string(FIND "${_LINE}" "\t" _TAB)
            if(NOT _TAB EQUAL -1)
                list(APPEND _GENERATION_LINES "${_LINE}")
                continue()
            endif()
            set(_PATH "${_LINE}")
            get_filename_component(_NAME "${_PATH}" NAME)
            if("${_NAME}" STREQUAL "sce_sourcemap.json")
                continue()
            endif()
            file(RELATIVE_PATH _RELATIVE "${_OUTPUT_DIR}" "${_PATH}")
            if("${_RELATIVE}" MATCHES "/")
                message(FATAL_ERROR
                    "sce-codegen --plan generate for ${_SCP_DOCUMENT} (${_LANGUAGE}) names "
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

        # Each child document's generation: its files, and which generation
        # writes the document it reads (the run, or a generation planned before
        # it), whose machine its command waits for.
        foreach(_LINE ${_GENERATION_LINES})
            string(FIND "${_LINE}" "\t" _TAB)
            string(SUBSTRING "${_LINE}" 0 ${_TAB} _CHILD_DOCUMENT)
            math(EXPR _AFTER "${_TAB} + 1")
            string(SUBSTRING "${_LINE}" ${_AFTER} -1 _PATH)
            get_filename_component(_CHILD_NAME "${_CHILD_DOCUMENT}" NAME)
            string(REGEX REPLACE "\\.scxml$" "" _CHILD "${_CHILD_NAME}")
            get_filename_component(_NAME "${_PATH}" NAME)
            file(RELATIVE_PATH _RELATIVE "${_OUTPUT_DIR}" "${_PATH}")
            if("${_RELATIVE}" MATCHES "/")
                message(FATAL_ERROR
                    "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) names "
                    "${_PATH} for ${_CHILD_NAME}, which is not directly in ${_OUTPUT_DIR}: "
                    "sce_codegen_plan declares the outputs of a flat output directory")
            endif()
            if(NOT "${_CHILD}" IN_LIST _GENERATIONS)
                list(APPEND _GENERATIONS "${_CHILD}")
                set(_GENERATION_${_CHILD}_DOCUMENT "${_CHILD_DOCUMENT}")
                set(_GENERATION_${_CHILD}_FILES "")
            endif()
            if(NOT "${_NAME}" STREQUAL "sce_sourcemap.json")
                list(APPEND _GENERATION_${_CHILD}_FILES "${_NAME}")
            endif()
        endforeach()
        set(_WRITERS_OF "${_STEM}")
        set(_WRITER_FILES_${_STEM} ${_FILES})
        foreach(_CHILD ${_GENERATIONS})
            set(_WRITER "")
            # A document staged from CHILDREN_HELD_IN is there before the build
            # starts: no generation writes it, and its command waits for none.
            if("${_OUTPUT_DIR}/${_CHILD}.scxml" IN_LIST _STAGED)
                set(_GENERATION_${_CHILD}_WRITER "")
                list(APPEND _WRITERS_OF "${_CHILD}")
                set(_WRITER_FILES_${_CHILD} ${_GENERATION_${_CHILD}_FILES})
                continue()
            endif()
            foreach(_CANDIDATE ${_WRITERS_OF})
                if("${_CHILD}.scxml" IN_LIST _WRITER_FILES_${_CANDIDATE})
                    _sce_codegen_plan_machine(_WRITER_MACHINE _UNUSED_DEPFILE
                        "${_CANDIDATE}" "${_LANGUAGE}")
                    list(GET _WRITER_MACHINE 0 _WRITER_HEADER)
                    set(_WRITER "${_OUTPUT_DIR}/${_WRITER_HEADER}")
                    break()
                endif()
            endforeach()
            if("${_WRITER}" STREQUAL "")
                message(FATAL_ERROR
                    "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) plans "
                    "the generation of ${_CHILD}.scxml, which neither the run nor a "
                    "generation planned before it writes")
            endif()
            set(_GENERATION_${_CHILD}_WRITER "${_WRITER}")
            list(APPEND _WRITERS_OF "${_CHILD}")
            set(_WRITER_FILES_${_CHILD} ${_GENERATION_${_CHILD}_FILES})
        endforeach()

        # Every child document the run writes or stages is one whose machine
        # the run's machine names, so a plan asked for the children's
        # generations holds one for each: a child without one is a header no
        # rule writes.
        if(_SCP_CHILD_GENERATIONS)
            foreach(_CHILD ${_CHILDREN})
                if(NOT "${_CHILD}" IN_LIST _GENERATIONS)
                    message(FATAL_ERROR
                        "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) "
                        "names the child document ${_CHILD}.scxml and plans no generation "
                        "of it, so the machine the run's machine names has no rule")
                endif()
            endforeach()
        endif()
    endif()

    _sce_codegen_plan_machine(_MACHINE _DEPFILE_NAME "${_STEM}" "${_LANGUAGE}")
    set(_DEPFILE "${_OUTPUT_DIR}/${_DEPFILE_NAME}")
    foreach(_NAMED ${_MACHINE})
        if(NOT "${_NAMED}" IN_LIST _FILES)
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${_STEM} (${_LANGUAGE}) does not name "
                "${_NAMED}, a file its generate command is declared to produce")
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

    # What another plan of the same claim declared is that plan's: its byproducts
    # are not declared again, and a machine it declared is generated by its
    # command, not by a second one.
    set(_DECLARED_ELSEWHERE FALSE)
    if(_CLAIM)
        get_property(_CLAIMED GLOBAL PROPERTY "${_CLAIM}")
        list(GET _OUTPUTS 0 _HEADER)
        if("${_HEADER}" IN_LIST _CLAIMED)
            set(_DECLARED_ELSEWHERE TRUE)
        endif()
        set(_UNCLAIMED "")
        foreach(_FILE ${_BYPRODUCTS})
            if(NOT "${_FILE}" IN_LIST _CLAIMED)
                list(APPEND _UNCLAIMED "${_FILE}")
            endif()
        endforeach()
        set(_BYPRODUCTS "${_UNCLAIMED}")
        if(NOT _DECLARED_ELSEWHERE)
            set_property(GLOBAL APPEND PROPERTY "${_CLAIM}" ${_OUTPUTS} ${_BYPRODUCTS})
        endif()
    endif()

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
    set(${PREFIX}_GENERATIONS "${_GENERATIONS}" PARENT_SCOPE)
    set(${PREFIX}_OUTPUTS "${_OUTPUTS}" PARENT_SCOPE)
    set(${PREFIX}_BYPRODUCTS "${_BYPRODUCTS}" PARENT_SCOPE)
    set(${PREFIX}_BYPRODUCTS_ARG "${_BYPRODUCTS_ARG}" PARENT_SCOPE)
    set(${PREFIX}_DEPFILE "${_DEPFILE}" PARENT_SCOPE)
    set(${PREFIX}_HEADERS "${_HEADERS}" PARENT_SCOPE)
    set(${PREFIX}_SOURCES "${_SOURCES}" PARENT_SCOPE)
    set(${PREFIX}_DECLARED_ELSEWHERE "${_DECLARED_ELSEWHERE}" PARENT_SCOPE)
    set(${PREFIX}_DOCUMENT "${_DOCUMENT}" PARENT_SCOPE)
    set(${PREFIX}_DEPENDS "${_DEPENDS}" PARENT_SCOPE)
    # What a plan FROM this one reads: where and how it was asked, and each
    # generation it holds.
    set(${PREFIX}_OUTPUT_DIR "${_OUTPUT_DIR}" PARENT_SCOPE)
    set(${PREFIX}_LANGUAGE "${_LANGUAGE}" PARENT_SCOPE)
    set(${PREFIX}_CLAIM "${_CLAIM}" PARENT_SCOPE)
    foreach(_CHILD ${_GENERATIONS})
        set(${PREFIX}_GENERATION_${_CHILD}_DOCUMENT "${_GENERATION_${_CHILD}_DOCUMENT}"
            PARENT_SCOPE)
        set(${PREFIX}_GENERATION_${_CHILD}_FILES "${_GENERATION_${_CHILD}_FILES}" PARENT_SCOPE)
        set(${PREFIX}_GENERATION_${_CHILD}_WRITER "${_GENERATION_${_CHILD}_WRITER}" PARENT_SCOPE)
    endforeach()
endfunction()
