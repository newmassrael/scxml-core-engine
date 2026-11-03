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
# the test tree declares every generation it runs through
# `sce_codegen_generation`, which asks it and declares the command, and
# `the_conformance_cmake_route_declares_what_the_generator_plans` holds all
# three to that.
#
# A generation with a deploy writes the machine and the transport the deploy
# routes it through, in one run: the generator decides which files a run
# writes, and a plan asked with the deploy names both. The test tree used to
# split such a generation into two commands, one declaring the machine and
# one declaring the transport with `--transport-only`, so that an edit to the
# deploy would not touch the machine; the machine's command read the deploy
# too, and wrote the transport as well, so the transport had two producers and
# one of them undeclared.
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

# What a generation's command is given to format what it writes
# (`sce_codegen_format_args`), which `sce_codegen_generation` gives every one.
include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

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
#               (`sce-codegen --plan-children`), with the run's language,
#               output directory and input root and the arguments the plan
#               states for it: `--as-child` for a child its parent starts in
#               its own process, `--deploy <the run's deploy>` for an in-line
#               child the run's deploy placed on a partition of its own
#               (SCE_MESH.md §9.6.6 rule 3). Each is declared by
#               `sce_codegen_plan(<P> FROM <PREFIX> CHILD <name>)`, its command
#               given `<P>_ARGS`; a file two generations would write is the
#               first's.
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
#                             (the deploy a generation on a partition of its
#                             own reads is one the generation that writes the
#                             document depends on, which rewrites it)
#
#   Sets, in the caller's scope:
#     <PREFIX>_DOCUMENT       the document the run reads: DOCUMENT, or for a
#                             plan FROM another, the child document
#     <PREFIX>_DEPENDS        what the command depends on before its depfile
#                             is written: the document (unless it is a
#                             library name) and each document staged from
#                             CHILDREN_HELD_IN, or for a plan FROM another,
#                             as above
#     <PREFIX>_INPUT_ROOT     the input root the run's generations are planned
#                             with: INPUT_ROOT, or the document's directory
#                             when none is given (empty for a library name),
#                             or for a plan FROM another, that plan's. A
#                             child's generation is given it, so its source
#                             set is the one it was planned with
#     <PREFIX>_ARGS           the arguments the run is given beside its
#                             document, language, output directory, input root,
#                             formatting and depfile: ARGS, or for a plan FROM
#                             another, the ones the generator states for the
#                             child's generation, so a command runs the
#                             generation that was planned
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
        set(_INPUT_ROOT "${${_SCP_FROM}_INPUT_ROOT}")
        set(_DOCUMENT "${${_FROM}_DOCUMENT}")
        set(_ARGS "${${_FROM}_ARGS}")
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
        set(_ARGS "${_SCP_ARGS}")
        # The language is judged before the generator is asked.
        _sce_codegen_plan_machine(_MACHINE _DEPFILE_NAME "${_STEM}" "${_LANGUAGE}")

        set(_INPUT_ROOT_ARGS "")
        set(_INPUT_ROOT "${_SCP_INPUT_ROOT}")
        set(_DOCUMENT "${_SCP_DOCUMENT}")
        if(_SCP_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_SCP_INPUT_ROOT}")
        elseif(NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
            # The root the generator takes when it is given none: the
            # document's own directory, which a child's generation, reading
            # its document from the output directory, is given explicitly.
            get_filename_component(_INPUT_ROOT "${_SCP_DOCUMENT}" DIRECTORY)
        endif()
        if(NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
            set(_DEPENDS "${_SCP_DOCUMENT}")
        endif()
        set(_CHILD_GENERATIONS_ARGS "")
        if(_SCP_CHILD_GENERATIONS)
            set(_CHILD_GENERATIONS_ARGS --plan-children)
        endif()
        # `--no-format`: formatting changes bytes and never which files, and a
        # plan is asked once per generation when the tree is configured, so it
        # neither waits for clang-format on every file nor needs one there.
        execute_process(
            COMMAND "${SCE_CODEGEN}" --plan ${_CHILD_GENERATIONS_ARGS}
                    generate "${_SCP_DOCUMENT}"
                    -l "${_LANGUAGE}" -o "${_OUTPUT_DIR}"
                    ${_INPUT_ROOT_ARGS}
                    --no-format
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
        # A staged child is read by the run, and is no file of the build's.
        list(APPEND _DEPENDS ${_STAGED})

        # Each child document's generation: the arguments it is run with, its
        # files, and which generation writes the document it reads (the run, or
        # a generation planned before it), whose machine its command waits for.
        # A generation's first line states its arguments, `<child document>`,
        # an empty field, and each argument after a tab of its own; each line
        # after it is one file, `<child document>\t<file>`.
        foreach(_LINE ${_GENERATION_LINES})
            string(FIND "${_LINE}" "\t" _TAB)
            string(SUBSTRING "${_LINE}" 0 ${_TAB} _CHILD_DOCUMENT)
            math(EXPR _AFTER "${_TAB} + 1")
            string(SUBSTRING "${_LINE}" ${_AFTER} -1 _PATH)
            get_filename_component(_CHILD_NAME "${_CHILD_DOCUMENT}" NAME)
            string(REGEX REPLACE "\\.scxml$" "" _CHILD "${_CHILD_NAME}")
            string(FIND "${_PATH}" "\t" _SECOND_TAB)
            if(_SECOND_TAB EQUAL 0)
                if("${_CHILD}" IN_LIST _GENERATIONS)
                    message(FATAL_ERROR
                        "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) "
                        "states the arguments of the generation of ${_CHILD_NAME} twice")
                endif()
                list(APPEND _GENERATIONS "${_CHILD}")
                set(_GENERATION_${_CHILD}_DOCUMENT "${_CHILD_DOCUMENT}")
                set(_GENERATION_${_CHILD}_FILES "")
                string(SUBSTRING "${_PATH}" 1 -1 _ARGUMENTS)
                string(REPLACE "\t" ";" _GENERATION_${_CHILD}_ARGS "${_ARGUMENTS}")
                continue()
            endif()
            if(NOT "${_CHILD}" IN_LIST _GENERATIONS)
                message(FATAL_ERROR
                    "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) names "
                    "${_PATH} for ${_CHILD_NAME} before the arguments of its generation. "
                    "A sce-codegen older than the arguments line answers so: rebuild it "
                    "from this tree.")
            endif()
            get_filename_component(_NAME "${_PATH}" NAME)
            file(RELATIVE_PATH _RELATIVE "${_OUTPUT_DIR}" "${_PATH}")
            if("${_RELATIVE}" MATCHES "/")
                message(FATAL_ERROR
                    "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) names "
                    "${_PATH} for ${_CHILD_NAME}, which is not directly in ${_OUTPUT_DIR}: "
                    "sce_codegen_plan declares the outputs of a flat output directory")
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
        #
        # The document of an in-line child is written with its machine, by the
        # run, unless the run's deploy placed the child on a partition of its
        # own (SCE_MESH.md §9.6.6 rule 3): then the machine is the generation
        # of that partition (and of a child below it, a file of that
        # generation), and the plan holds that generation too.
        if(_SCP_CHILD_GENERATIONS)
            foreach(_CHILD ${_CHILDREN})
                if(NOT "${_CHILD}" IN_LIST _GENERATIONS)
                    message(FATAL_ERROR
                        "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) "
                        "names the child document ${_CHILD}.scxml and plans no generation "
                        "of it, so the machine the run's machine names has no rule")
                endif()
            endforeach()
            set(_PLANNED_FILES ${_FILES})
            foreach(_CHILD ${_GENERATIONS})
                list(APPEND _PLANNED_FILES ${_GENERATION_${_CHILD}_FILES})
            endforeach()
            foreach(_CHILD ${_SYNTH})
                _sce_codegen_plan_machine(_SYNTH_MACHINE _UNUSED_DEPFILE
                    "${_CHILD}" "${_LANGUAGE}")
                list(GET _SYNTH_MACHINE 0 _SYNTH_HEADER)
                if(NOT "${_SYNTH_HEADER}" IN_LIST _PLANNED_FILES)
                    message(FATAL_ERROR
                        "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) "
                        "writes the document of the in-line child ${_CHILD}.scxml and "
                        "neither writes its machine nor plans the generation that does")
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

    _sce_codegen_plan_publish(${PREFIX} _)
endfunction()

# _sce_codegen_plan_publish(<TO> <FROM>)
#
#   Set, in the scope of the caller of the function this is expanded in, each
#   variable of a plan's answer as <TO>_<NAME>, from the variable <FROM><NAME>
#   of that function's scope: `sce_codegen_plan` publishes its own locals
#   (`<FROM>` is `_`), and `sce_codegen_generation` passes on the answer it
#   was given. One list, so a variable added to the answer reaches every
#   caller. A macro, so that PARENT_SCOPE is the caller of that function.
#
#   The answer is every variable `sce_codegen_plan` documents, and, for each
#   generation a CHILD_GENERATIONS plan holds, what a plan FROM it reads: its
#   document, its files, its arguments and the machine of the generation
#   that writes its document.
macro(_sce_codegen_plan_publish TO FROM)
    foreach(_SCPP_NAME FILES SYNTH CHILDREN STAGED GENERATIONS OUTPUTS BYPRODUCTS
            BYPRODUCTS_ARG DEPFILE HEADERS SOURCES DECLARED_ELSEWHERE DOCUMENT DEPENDS
            ARGS STEM OUTPUT_DIR LANGUAGE CLAIM INPUT_ROOT)
        set(${TO}_${_SCPP_NAME} "${${FROM}${_SCPP_NAME}}" PARENT_SCOPE)
    endforeach()
    foreach(_SCPP_CHILD ${${FROM}GENERATIONS})
        foreach(_SCPP_NAME DOCUMENT FILES ARGS WRITER)
            set(${TO}_GENERATION_${_SCPP_CHILD}_${_SCPP_NAME}
                "${${FROM}GENERATION_${_SCPP_CHILD}_${_SCPP_NAME}}" PARENT_SCOPE)
        endforeach()
    endforeach()
endmacro()

# sce_codegen_generation(<PREFIX>
#     DOCUMENT <document> OUTPUT_DIR <dir> LANGUAGE <cpp|c11>
#     [STEM <stem>]
#     [DEPLOY <deploy.yaml>]
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [CHILD_GENERATIONS]
#     [CLAIM <global property>]
#     [ARGS <arg>...]
#     [COMMENT <text>])
#
#   One generation of DOCUMENT into OUTPUT_DIR, declared from its plan: ask
#   `sce_codegen_plan` which files the run writes, then declare the command
#   that runs it with exactly those as its OUTPUT and BYPRODUCTS, the depfile
#   it asks the generator for as its DEPFILE, and what it reads before that
#   depfile exists (the document, the deploy, the staged children, the
#   generator) as its DEPENDS. Every file of the run has this command as its
#   one producer, and no file name is written at the call: a run with a
#   deploy declares the transport it writes beside the machine because the
#   plan names it.
#
#   STEM     The machine's name, which names its files: the document's name
#            without `.scxml` when not given.
#   DEPLOY   The deploy the run is generated with (`--deploy`), which the
#            plan is asked with too and the command depends on.
#   ARGS     The rest of the arguments that change what the run emits
#            (`--partition <name>` ...). Formatting is the build's
#            (`sce_codegen_format_args`) and is given to the command alone.
#   COMMENT  What the build prints when it runs the command.
#   The others are `sce_codegen_plan`'s.
#
# sce_codegen_generation(<PREFIX> FROM <plan> CHILD <name> [COMMENT <text>])
#
#   The generation of the child document <name> the plan <plan> holds (asked
#   with CHILD_GENERATIONS), declared the same way and run with the arguments
#   and input root the plan states for it: it depends on its document and on
#   the machine of the generation that writes it.
#
#   Either form sets, in the caller's scope, every variable
#   `sce_codegen_plan` sets: `<PREFIX>_HEADERS` and `<PREFIX>_SOURCES` are
#   what a target compiles of the run, and `<PREFIX>_GENERATIONS` names the
#   children a CHILD_GENERATIONS plan holds, each declared by the second form.
function(sce_codegen_generation PREFIX)
    cmake_parse_arguments(PARSE_ARGV 1 _SCGA "CHILD_GENERATIONS"
        "STEM;DOCUMENT;OUTPUT_DIR;LANGUAGE;DEPLOY;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM;FROM;CHILD;COMMENT"
        "ARGS")
    if(_SCGA_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_generation(${PREFIX}): unexpected argument(s): "
            "${_SCGA_UNPARSED_ARGUMENTS}")
    endif()

    set(_DEPENDS "")
    set(_INPUT_ROOT_ARGS "")
    if(_SCGA_FROM)
        foreach(_GIVEN STEM DOCUMENT OUTPUT_DIR LANGUAGE DEPLOY INPUT_ROOT CHILDREN_HELD_IN
                CLAIM ARGS)
            if(NOT "${_SCGA_${_GIVEN}}" STREQUAL "")
                message(FATAL_ERROR
                    "sce_codegen_generation(${PREFIX} FROM ${_SCGA_FROM}): ${_GIVEN} is the "
                    "plan's to state, not this call's")
            endif()
        endforeach()
        if(_SCGA_CHILD_GENERATIONS)
            message(FATAL_ERROR
                "sce_codegen_generation(${PREFIX} FROM ${_SCGA_FROM}): CHILD_GENERATIONS is "
                "the plan's to state")
        endif()
        sce_codegen_plan(_SCGA_PLAN FROM "${_SCGA_FROM}" CHILD "${_SCGA_CHILD}")
        # A child's document is read from the output directory, and the
        # source set it was planned with is the run's.
        if(_SCGA_PLAN_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_SCGA_PLAN_INPUT_ROOT}")
        endif()
    else()
        if(_SCGA_CHILD)
            message(FATAL_ERROR
                "sce_codegen_generation(${PREFIX}): CHILD names a generation of the plan "
                "FROM names")
        endif()
        foreach(_REQUIRED DOCUMENT OUTPUT_DIR LANGUAGE)
            if("${_SCGA_${_REQUIRED}}" STREQUAL "")
                message(FATAL_ERROR "sce_codegen_generation(${PREFIX}): ${_REQUIRED} is required")
            endif()
        endforeach()
        set(_STEM "${_SCGA_STEM}")
        if("${_STEM}" STREQUAL "")
            get_filename_component(_STEM "${_SCGA_DOCUMENT}" NAME_WLE)
        endif()
        set(_RUN_ARGS "")
        if(_SCGA_DEPLOY)
            set(_RUN_ARGS --deploy "${_SCGA_DEPLOY}")
            list(APPEND _DEPENDS "${_SCGA_DEPLOY}")
        endif()
        list(APPEND _RUN_ARGS ${_SCGA_ARGS})
        set(_PLAN_OPTIONS "")
        if(_SCGA_CHILD_GENERATIONS)
            list(APPEND _PLAN_OPTIONS CHILD_GENERATIONS)
        endif()
        foreach(_PASSED INPUT_ROOT CHILDREN_HELD_IN CLAIM)
            if(NOT "${_SCGA_${_PASSED}}" STREQUAL "")
                list(APPEND _PLAN_OPTIONS ${_PASSED} "${_SCGA_${_PASSED}}")
            endif()
        endforeach()
        sce_codegen_plan(_SCGA_PLAN
            STEM "${_STEM}"
            DOCUMENT "${_SCGA_DOCUMENT}"
            OUTPUT_DIR "${_SCGA_OUTPUT_DIR}"
            LANGUAGE "${_SCGA_LANGUAGE}"
            ${_PLAN_OPTIONS}
            ARGS ${_RUN_ARGS})
        if(_SCGA_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_SCGA_INPUT_ROOT}")
        endif()
    endif()
    list(APPEND _DEPENDS ${_SCGA_PLAN_DEPENDS})

    set(_COMMENT "${_SCGA_COMMENT}")
    if("${_COMMENT}" STREQUAL "")
        set(_COMMENT "Generating ${_SCGA_PLAN_STEM} (${_SCGA_PLAN_LANGUAGE})")
    endif()

    # A machine another plan of the same claim declared is that plan's
    # command's to write.
    if(NOT _SCGA_PLAN_DECLARED_ELSEWHERE)
        sce_codegen_format_args(_FORMAT_ARGS)
        add_custom_command(
            OUTPUT ${_SCGA_PLAN_OUTPUTS}
            COMMAND "${SCE_CODEGEN}" generate "${_SCGA_PLAN_DOCUMENT}"
                    ${_SCGA_PLAN_ARGS}
                    -l "${_SCGA_PLAN_LANGUAGE}" -o "${_SCGA_PLAN_OUTPUT_DIR}"
                    ${_INPUT_ROOT_ARGS}
                    ${_FORMAT_ARGS}
                    --write-deps "${_SCGA_PLAN_DEPFILE}"
            DEPENDS ${_DEPENDS} "${SCE_CODEGEN}"
            DEPFILE "${_SCGA_PLAN_DEPFILE}"
            ${_SCGA_PLAN_BYPRODUCTS_ARG}
            COMMENT "${_COMMENT}"
            VERBATIM
        )
    endif()

    _sce_codegen_plan_publish(${PREFIX} _SCGA_PLAN_)
endfunction()
