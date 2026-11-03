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
#
# ── One answer per input state, many asked at once ──────────────────────
#
# A plan is the whole generation run in memory, and a configure of the test
# tree with the Mesh fixtures asks several hundred of them: asked one after
# another, each by a process of its own, they took 207 s of an 8-core
# machine's configure (2026-11), paid again by every reconfigure and every CI
# lane that configures. Two things remove that cost without changing an
# answer.
#
# An answer is a function of the generator, its arguments, the environment
# the generator reads its installation from, and the files it read. Each is
# kept in the build tree (`<build>/sce-codegen-plans/entries/<key>.plan`),
# under the SHA-256 of the generator's path, that environment and the
# arguments, beside the list of the files the plan read (`<key>.reads`,
# `sce-codegen --plan-reads`, the prerequisites the depfile of the same run
# names: its document, what it includes and imports, its source set, its
# deploy, the templates and the generator; the files the plan produced left
# out; a file it looked for and did not find written `?<path>`). A kept
# answer is reused only when neither the generator nor any file of its list
# is newer than it, compared as CMake compares files (`IS_NEWER_THAN`, which
# calls a tie newer), and every file it did not find is still absent. The
# generator dates every answer of a batch when the batch begins, before any
# plan read anything and strictly after every file written before it, so a
# file that changed while a plan ran is never older than the answer and a
# document staged just before the batch never ties with it. An answer with
# no list is never reused.
#
# Every plan not kept is asked of `sce-codegen plan-batch`, which runs each
# request as the `--plan generate` run a single ask would make, as many at a
# time as there are processors (`SCE_CODEGEN_PLAN_JOBS`), and keeps each
# answer where the next ask finds it. A route asks every plan it will declare
# before it declares one ("request first, declare after"):
#   - when this module is first included, every answer the last configure of
#     this build tree asked for that is no longer current is asked again in
#     one batch, which is every plan after the generator is rebuilt
#     (`_sce_codegen_plan_refresh`);
#   - a route queues the plans of its generations
#     (`sce_codegen_plan_request`, `sce_codegen_generation_request`) and sends
#     them together (`sce_codegen_plan_prefetch`) before declaring them: the
#     conformance route for every fixture the registry lists, and, after their
#     last registration, the C11 conformance and C++ integration fixtures; the
#     C11 integration fixtures and the Mesh fixtures, whose targets compile
#     each one's files right after it is registered, declare at the end of
#     their directory (`cmake_language(DEFER)`) and hand their callers a
#     generator expression for the files;
#   - a plan asked otherwise and not kept is sent alone, when it is asked.
# At the end of the configure, the answers no plan asked for are removed,
# the list of the asked ones is kept for the next configure, and one line
# says how many plans were asked, how many were kept, and how many the
# generator was sent (`_sce_codegen_plan_settle`), followed, when a kept
# answer was not current, by why (`stale.txt` holds every reason).
#
# What the cache cannot see is what no list names, and a file put back with
# an older date than the answer. A depfile has the same blind spots;
# removing `<build>/sce-codegen-plans` asks every plan again.

include_guard(GLOBAL)

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCECodegenPlan.cmake: SCE_CODEGEN unset. Include SCEStaticW3CTest.cmake "
        "first (it resolves the binary through SCEFindCodegen.cmake).")
endif()

# What a generation's command is given to format what it writes
# (`sce_codegen_format_args`), which `sce_codegen_generation` gives every one.
include(${CMAKE_CURRENT_LIST_DIR}/SCEClangFormat.cmake)

set(SCE_CODEGEN_PLAN_JOBS "" CACHE STRING
    "How many plans sce-codegen runs at once when the tree is configured; empty runs one per processor")

# _sce_codegen_plan_cache_dir(<VAR>)
#
#   Where the answers of this build tree's plans are kept.
function(_sce_codegen_plan_cache_dir VAR)
    set(${VAR} "${CMAKE_BINARY_DIR}/sce-codegen-plans" PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_count(<COUNTER> <BY>)
#
#   Add BY to one of the counts `_sce_codegen_plan_settle` reports.
function(_sce_codegen_plan_count COUNTER BY)
    get_property(_NOW GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_${COUNTER}")
    if("${_NOW}" STREQUAL "")
        set(_NOW 0)
    endif()
    math(EXPR _NOW "${_NOW} + ${BY}")
    set_property(GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_${COUNTER}" "${_NOW}")
endfunction()

# _sce_codegen_plan_arguments(<VAR> <document> <language> <output dir>
#     <input root> <child generations> [<argument>...])
#
#   The arguments `sce-codegen` is run with to plan one generation, after the
#   program: the one spelling of a plan's request, which `sce_codegen_plan`
#   asks and `sce_codegen_plan_request` asks ahead. An empty input root passes
#   none; a true <child generations> asks for the plans of the children too
#   (`--plan-children`).
#
#   `--no-format`: formatting changes bytes and never which files, and a plan
#   is asked once per generation when the tree is configured, so it neither
#   waits for clang-format on every file nor needs one there.
function(_sce_codegen_plan_arguments VAR DOCUMENT LANGUAGE OUTPUT_DIR INPUT_ROOT CHILD_GENERATIONS)
    set(_ARGUMENTS --plan)
    if(CHILD_GENERATIONS)
        list(APPEND _ARGUMENTS --plan-children)
    endif()
    list(APPEND _ARGUMENTS generate "${DOCUMENT}" -l "${LANGUAGE}" -o "${OUTPUT_DIR}")
    if(INPUT_ROOT)
        list(APPEND _ARGUMENTS --input-root "${INPUT_ROOT}")
    endif()
    list(APPEND _ARGUMENTS --no-format ${ARGN})
    set(${VAR} "${_ARGUMENTS}" PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_entry(<ENTRY_VAR> <LINE_VAR> <argument>...)
#
#   Where the answer of the plan asked with <argument>s is kept
#   (`<cache>/entries/<key>`, each file of it `<key>.<kind>`), and the line
#   that asks `sce-codegen plan-batch` for it (`<key>`, then each argument,
#   separated by tabs).
#
#   <key> is the SHA-256 of what the answer depends on besides the files it
#   read: the generator's path, the environment the generator finds its
#   installation and its confinement through, and the arguments. Each
#   variable of `_ENVIRONMENT` is one the generator reads when it plans, and
#   `the_conformance_cmake_route_declares_what_the_generator_plans` holds the
#   list to the generator's sources.
function(_sce_codegen_plan_entry ENTRY_VAR LINE_VAR)
    foreach(_ARGUMENT IN LISTS ARGN)
        if("${_ARGUMENT}" MATCHES "[\t\n]")
            message(FATAL_ERROR
                "sce_codegen_plan: the argument \"${_ARGUMENT}\" holds a tab or a line "
                "break, which a request to sce-codegen plan-batch cannot carry")
        endif()
    endforeach()
    set(_ENVIRONMENT SCE_FILE_ROOT SCE_SCHEMAS_DIR SCE_TEMPLATE_DIR SCE_WORKSPACE_ROOT)
    set(_IDENTITY "${SCE_CODEGEN}")
    foreach(_NAME IN LISTS _ENVIRONMENT)
        string(APPEND _IDENTITY "\n${_NAME}=$ENV{${_NAME}}")
    endforeach()
    string(JOIN "\t" _REQUEST ${ARGN})
    string(SHA256 _KEY "${_IDENTITY}\n${_REQUEST}")
    _sce_codegen_plan_cache_dir(_CACHE)
    set(${ENTRY_VAR} "${_CACHE}/entries/${_KEY}" PARENT_SCOPE)
    set(${LINE_VAR} "${_KEY}\t${_REQUEST}" PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_fresh(<VAR> <entry>)
#
#   Set VAR to TRUE when the answer kept at <entry> is the one the generator
#   would give now: it is there with the list of the files its plan read, and
#   neither the generator nor any of those files is newer than it, and every
#   file the plan looked for and did not find (`?<path>`) is still absent. A
#   file of the list that is gone is newer, as `IS_NEWER_THAN` reads a missing
#   file, and so is one dated the same instant as the answer.
#
#   An answer that is not current says why: the reason is kept for
#   `_sce_codegen_plan_settle` to report, so a configure that asks the
#   generator again names what made it.
function(_sce_codegen_plan_fresh VAR ENTRY)
    set(${VAR} FALSE PARENT_SCOPE)
    set(_WHY "")
    if(NOT EXISTS "${ENTRY}.plan")
        set(_WHY "no answer is kept")
    elseif(NOT EXISTS "${ENTRY}.reads")
        set(_WHY "the answer has no list of what its plan read")
    elseif("${SCE_CODEGEN}" IS_NEWER_THAN "${ENTRY}.plan")
        set(_WHY "${SCE_CODEGEN} is newer")
    else()
        file(STRINGS "${ENTRY}.reads" _READS ENCODING UTF-8)
        foreach(_READ IN LISTS _READS)
            if("${_READ}" MATCHES "^\\?(.*)$")
                if(EXISTS "${CMAKE_MATCH_1}")
                    set(_WHY "${CMAKE_MATCH_1}, absent when it was planned, is there")
                    break()
                endif()
            elseif(NOT EXISTS "${_READ}")
                set(_WHY "${_READ}, which it read, is gone")
                break()
            elseif("${_READ}" IS_NEWER_THAN "${ENTRY}.plan")
                if("${ENTRY}.plan" IS_NEWER_THAN "${_READ}")
                    set(_WHY "${_READ} is dated the same instant as the answer")
                else()
                    set(_WHY "${_READ} is newer")
                endif()
                break()
            endif()
        endforeach()
    endif()
    if(NOT "${_WHY}" STREQUAL "")
        set(_DOCUMENT "${ENTRY}")
        if(EXISTS "${ENTRY}.request")
            file(READ "${ENTRY}.request" _ASKED)
            if("${_ASKED}" MATCHES "\tgenerate\t([^\t\n]*)")
                set(_DOCUMENT "${CMAKE_MATCH_1}")
            endif()
        endif()
        set_property(GLOBAL APPEND PROPERTY _SCE_CODEGEN_PLAN_STALE "${_DOCUMENT}: ${_WHY}")
        return()
    endif()
    set(${VAR} TRUE PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_ask(<line>...)
#
#   Send every request given (a line of `_sce_codegen_plan_entry`) to one
#   `sce-codegen plan-batch`, which answers them `SCE_CODEGEN_PLAN_JOBS` at a
#   time and keeps each answer where `_sce_codegen_plan_entry` names it: what
#   the plan printed, or what a refused plan wrote on stderr. The one place
#   this module runs the generator.
function(_sce_codegen_plan_ask)
    list(LENGTH ARGN _SENT)
    if(_SENT EQUAL 0)
        return()
    endif()
    _sce_codegen_plan_cache_dir(_CACHE)
    file(MAKE_DIRECTORY "${_CACHE}/entries")
    string(JOIN "\n" _REQUESTS ${ARGN})
    file(WRITE "${_CACHE}/requests.txt" "${_REQUESTS}\n")
    set(_JOBS "")
    if(SCE_CODEGEN_PLAN_JOBS)
        set(_JOBS --jobs "${SCE_CODEGEN_PLAN_JOBS}")
    endif()
    execute_process(
        COMMAND "${SCE_CODEGEN}" plan-batch
                --requests "${_CACHE}/requests.txt"
                --results "${_CACHE}/entries"
                ${_JOBS}
        RESULT_VARIABLE _RESULT
        OUTPUT_VARIABLE _OUTPUT
        ERROR_VARIABLE _ERROR
    )
    if(NOT _RESULT EQUAL 0)
        message(FATAL_ERROR
            "sce-codegen plan-batch failed, so the outputs of the generations it was asked "
            "to plan cannot be declared. A sce-codegen older than plan-batch answers with a "
            "usage error: rebuild it from this tree.\n${_ERROR}")
    endif()
    _sce_codegen_plan_count(SENT ${_SENT})
    _sce_codegen_plan_count(BATCHES 1)
endfunction()

# _sce_codegen_plan_answer(<RESULT_VAR> <OUTPUT_VAR> <ERROR_VAR> <argument>...)
#
#   The generator's answer to the plan asked with <argument>s, as
#   `execute_process` would have given it: RESULT_VAR 0 and OUTPUT_VAR what
#   the plan printed, its trailing white space stripped, or RESULT_VAR 1 and
#   ERROR_VAR what the refused plan wrote on stderr. A kept answer that is
#   current is read; any other is asked of the generator now.
function(_sce_codegen_plan_answer RESULT_VAR OUTPUT_VAR ERROR_VAR)
    _sce_codegen_plan_entry(_ENTRY _LINE ${ARGN})
    get_filename_component(_KEY "${_ENTRY}" NAME)
    set_property(GLOBAL APPEND PROPERTY _SCE_CODEGEN_PLAN_ASKED_KEYS "${_KEY}")
    _sce_codegen_plan_count(ASKED 1)
    _sce_codegen_plan_fresh(_FRESH "${_ENTRY}")
    if(_FRESH)
        _sce_codegen_plan_count(KEPT 1)
    else()
        _sce_codegen_plan_ask("${_LINE}")
    endif()
    if(EXISTS "${_ENTRY}.plan")
        file(READ "${_ENTRY}.plan" _ANSWER)
        string(REGEX REPLACE "[ \t\r\n]+$" "" _ANSWER "${_ANSWER}")
        set(${RESULT_VAR} 0 PARENT_SCOPE)
        set(${OUTPUT_VAR} "${_ANSWER}" PARENT_SCOPE)
        set(${ERROR_VAR} "" PARENT_SCOPE)
    elseif(EXISTS "${_ENTRY}.error")
        file(READ "${_ENTRY}.error" _REFUSAL)
        set(${RESULT_VAR} 1 PARENT_SCOPE)
        set(${OUTPUT_VAR} "" PARENT_SCOPE)
        set(${ERROR_VAR} "${_REFUSAL}" PARENT_SCOPE)
    else()
        message(FATAL_ERROR
            "sce-codegen plan-batch kept no answer at ${_ENTRY} for the plan asked with: "
            "${ARGN}")
    endif()
endfunction()

# _sce_codegen_plan_refresh()
#
#   Ask again, in one batch, every plan the last configure of this build tree
#   asked for whose kept answer is no longer current, before any is asked one
#   by one: after the generator is rebuilt, that is every plan. A kept request
#   whose key its arguments no longer give (another generator, another
#   environment) is not this configure's to ask.
function(_sce_codegen_plan_refresh)
    _sce_codegen_plan_cache_dir(_CACHE)
    if(NOT EXISTS "${_CACHE}/asked.txt")
        return()
    endif()
    file(STRINGS "${_CACHE}/asked.txt" _KEYS ENCODING UTF-8)
    set(_STALE "")
    foreach(_KEY IN LISTS _KEYS)
        set(_ENTRY "${_CACHE}/entries/${_KEY}")
        if(NOT EXISTS "${_ENTRY}.request")
            continue()
        endif()
        _sce_codegen_plan_fresh(_FRESH "${_ENTRY}")
        if(_FRESH)
            continue()
        endif()
        file(READ "${_ENTRY}.request" _KEPT)
        string(REGEX REPLACE "\n$" "" _KEPT "${_KEPT}")
        string(REPLACE "\t" ";" _FIELDS "${_KEPT}")
        list(POP_FRONT _FIELDS _KEPT_KEY)
        _sce_codegen_plan_entry(_NOW _LINE ${_FIELDS})
        if("${_NOW}" STREQUAL "${_ENTRY}" AND "${_KEPT_KEY}" STREQUAL "${_KEY}")
            list(APPEND _STALE "${_LINE}")
        endif()
    endforeach()
    _sce_codegen_plan_ask(${_STALE})
endfunction()

# _sce_codegen_plan_settle()
#
#   At the end of the configure: keep the list of the plans it asked for,
#   for the next configure to refresh, remove every answer no plan asked for,
#   and say what the plans cost.
function(_sce_codegen_plan_settle)
    _sce_codegen_plan_cache_dir(_CACHE)
    get_property(_ASKED GLOBAL PROPERTY _SCE_CODEGEN_PLAN_ASKED_KEYS)
    list(REMOVE_DUPLICATES _ASKED)
    file(MAKE_DIRECTORY "${_CACHE}/entries")
    list(LENGTH _ASKED _ASKED_COUNT)
    if(_ASKED_COUNT GREATER 0)
        string(JOIN "\n" _TEXT ${_ASKED})
        file(WRITE "${_CACHE}/asked.txt" "${_TEXT}\n")
    else()
        file(WRITE "${_CACHE}/asked.txt" "")
    endif()
    file(GLOB _KEPT "${_CACHE}/entries/*")
    foreach(_FILE IN LISTS _KEPT)
        get_filename_component(_NAME "${_FILE}" NAME)
        string(FIND "${_NAME}" "." _DOT)
        string(SUBSTRING "${_NAME}" 0 ${_DOT} _KEY)
        if(NOT "${_KEY}" IN_LIST _ASKED)
            file(REMOVE "${_FILE}")
        endif()
    endforeach()
    foreach(_COUNTER ASKED KEPT SENT BATCHES)
        get_property(_COUNT_${_COUNTER} GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_${_COUNTER}")
        if("${_COUNT_${_COUNTER}}" STREQUAL "")
            set(_COUNT_${_COUNTER} 0)
        endif()
    endforeach()
    message(STATUS
        "SCE: generation plans: ${_COUNT_ASKED} asked, ${_COUNT_KEPT} answered from the "
        "cache; ${_COUNT_SENT} sent to sce-codegen in ${_COUNT_BATCHES} batch(es) "
        "(${_CACHE})")

    # Why each kept answer that was asked again was not current: every reason
    # in `stale.txt`, and the first few said, an answer never kept aside. A
    # configure that changed nothing has none to say.
    get_property(_STALE GLOBAL PROPERTY _SCE_CODEGEN_PLAN_STALE)
    list(REMOVE_DUPLICATES _STALE)
    string(JOIN "\n" _STALE_TEXT ${_STALE})
    file(WRITE "${_CACHE}/stale.txt" "${_STALE_TEXT}\n")
    set(_SAID "")
    foreach(_REASON IN LISTS _STALE)
        if(NOT "${_REASON}" MATCHES ": no answer is kept$")
            list(APPEND _SAID "${_REASON}")
        endif()
    endforeach()
    list(LENGTH _SAID _SAID_COUNT)
    if(_SAID_COUNT GREATER 0)
        list(SUBLIST _SAID 0 5 _FIRST)
        string(JOIN "\n    " _FIRST_TEXT ${_FIRST})
        message(STATUS
            "SCE: ${_SAID_COUNT} kept plan answer(s) were not current, among them:\n"
            "    ${_FIRST_TEXT}\n"
            "  every reason: ${_CACHE}/stale.txt")
    endif()
endfunction()

# sce_codegen_plan_request(STEM <stem> DOCUMENT <document> OUTPUT_DIR <dir>
#     LANGUAGE <cpp|c11>
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [CHILD_GENERATIONS]
#     [CLAIM <global property>]
#     [ARGS <arg>...])
#
#   Ask ahead for the plan a later `sce_codegen_plan(<PREFIX> ...)` with the
#   same options asks: queue its request, which `sce_codegen_plan_prefetch`
#   sends to the generator in one batch with every other request queued. The
#   options are `sce_codegen_plan`'s, so a route asks ahead with the very
#   options it plans with; the ones that say how an answer is declared and
#   not what the generator is asked (STEM, CHILDREN_HELD_IN, CLAIM) change no
#   request. The answer is read by `sce_codegen_plan` as any kept answer is,
#   so a request asked ahead that the plan does not match only costs a plan.
function(sce_codegen_plan_request)
    cmake_parse_arguments(PARSE_ARGV 0 _SCR "CHILD_GENERATIONS"
        "STEM;DOCUMENT;OUTPUT_DIR;LANGUAGE;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM"
        "ARGS")
    if(_SCR_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_plan_request: unexpected argument(s): ${_SCR_UNPARSED_ARGUMENTS}")
    endif()
    foreach(_REQUIRED STEM DOCUMENT OUTPUT_DIR LANGUAGE)
        if("${_SCR_${_REQUIRED}}" STREQUAL "")
            message(FATAL_ERROR "sce_codegen_plan_request: ${_REQUIRED} is required")
        endif()
    endforeach()
    # The language is judged as `sce_codegen_plan` judges it.
    _sce_codegen_plan_machine(_MACHINE _DEPFILE_NAME "${_SCR_STEM}" "${_SCR_LANGUAGE}")
    _sce_codegen_plan_arguments(_REQUEST "${_SCR_DOCUMENT}" "${_SCR_LANGUAGE}"
        "${_SCR_OUTPUT_DIR}" "${_SCR_INPUT_ROOT}" "${_SCR_CHILD_GENERATIONS}" ${_SCR_ARGS})
    _sce_codegen_plan_entry(_ENTRY _LINE ${_REQUEST})
    get_filename_component(_KEY "${_ENTRY}" NAME)
    set_property(GLOBAL APPEND PROPERTY _SCE_CODEGEN_PLAN_QUEUED "${_KEY}")
    set_property(GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_LINE_${_KEY}" "${_LINE}")
endfunction()

# sce_codegen_plan_prefetch()
#
#   Send every request `sce_codegen_plan_request` queued whose kept answer is
#   not current to the generator, in one batch, and empty the queue.
function(sce_codegen_plan_prefetch)
    get_property(_KEYS GLOBAL PROPERTY _SCE_CODEGEN_PLAN_QUEUED)
    set_property(GLOBAL PROPERTY _SCE_CODEGEN_PLAN_QUEUED "")
    list(REMOVE_DUPLICATES _KEYS)
    _sce_codegen_plan_cache_dir(_CACHE)
    set(_STALE "")
    foreach(_KEY IN LISTS _KEYS)
        _sce_codegen_plan_fresh(_FRESH "${_CACHE}/entries/${_KEY}")
        if(NOT _FRESH)
            get_property(_LINE GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_LINE_${_KEY}")
            list(APPEND _STALE "${_LINE}")
        endif()
    endforeach()
    _sce_codegen_plan_ask(${_STALE})
endfunction()

# Once per configure, before any plan is asked: bring up to date what the last
# configure asked for, and settle the cache when the configure ends. A script
# (`cmake -P`) has no configure to end.
if(NOT CMAKE_SCRIPT_MODE_FILE)
    _sce_codegen_plan_refresh()
    cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}" CALL _sce_codegen_plan_settle)
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

        set(_INPUT_ROOT "${_SCP_INPUT_ROOT}")
        set(_DOCUMENT "${_SCP_DOCUMENT}")
        if(NOT _SCP_INPUT_ROOT AND NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
            # The root the generator takes when it is given none: the
            # document's own directory, which a child's generation, reading
            # its document from the output directory, is given explicitly.
            get_filename_component(_INPUT_ROOT "${_SCP_DOCUMENT}" DIRECTORY)
        endif()
        if(NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
            set(_DEPENDS "${_SCP_DOCUMENT}")
        endif()
        # The answer kept for this request while it is current, the
        # generator's otherwise (`_sce_codegen_plan_answer`).
        _sce_codegen_plan_arguments(_REQUEST "${_SCP_DOCUMENT}" "${_LANGUAGE}"
            "${_OUTPUT_DIR}" "${_SCP_INPUT_ROOT}" "${_SCP_CHILD_GENERATIONS}" ${_SCP_ARGS})
        _sce_codegen_plan_answer(_PLAN_RESULT _PLAN_OUTPUT _PLAN_ERROR ${_REQUEST})
        if(NOT _PLAN_RESULT EQUAL 0)
            message(FATAL_ERROR
                "sce-codegen --plan generate failed for ${_SCP_DOCUMENT} (${_LANGUAGE}), "
                "so the outputs of its generation cannot be declared. A sce-codegen older "
                "than --plan (or, asked with CHILD_GENERATIONS, than --plan-children, or "
                "than --plan-reads) answers with a usage error: rebuild it from this tree.\n"
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

# _sce_codegen_generation_plan_options(<VAR> <PARSED> <CALLER>)
#
#   The options `sce_codegen_plan` is asked with for one generation whose
#   options `cmake_parse_arguments` parsed into `<PARSED>_*` in the caller's
#   scope: the one spelling for `sce_codegen_generation`, which asks the plan,
#   and `sce_codegen_generation_request`, which asks it ahead. The stem is the
#   document's name when not given, and the deploy is passed as the run's
#   `--deploy`. CALLER names the call in a refusal.
function(_sce_codegen_generation_plan_options VAR PARSED CALLER)
    foreach(_REQUIRED DOCUMENT OUTPUT_DIR LANGUAGE)
        if("${${PARSED}_${_REQUIRED}}" STREQUAL "")
            message(FATAL_ERROR "${CALLER}: ${_REQUIRED} is required")
        endif()
    endforeach()
    set(_STEM "${${PARSED}_STEM}")
    if("${_STEM}" STREQUAL "")
        get_filename_component(_STEM "${${PARSED}_DOCUMENT}" NAME_WLE)
    endif()
    set(_OPTIONS
        STEM "${_STEM}"
        DOCUMENT "${${PARSED}_DOCUMENT}"
        OUTPUT_DIR "${${PARSED}_OUTPUT_DIR}"
        LANGUAGE "${${PARSED}_LANGUAGE}")
    if(${PARSED}_CHILD_GENERATIONS)
        list(APPEND _OPTIONS CHILD_GENERATIONS)
    endif()
    foreach(_PASSED INPUT_ROOT CHILDREN_HELD_IN CLAIM)
        if(NOT "${${PARSED}_${_PASSED}}" STREQUAL "")
            list(APPEND _OPTIONS ${_PASSED} "${${PARSED}_${_PASSED}}")
        endif()
    endforeach()
    list(APPEND _OPTIONS ARGS)
    if(${PARSED}_DEPLOY)
        list(APPEND _OPTIONS --deploy "${${PARSED}_DEPLOY}")
    endif()
    list(APPEND _OPTIONS ${${PARSED}_ARGS})
    set(${VAR} "${_OPTIONS}" PARENT_SCOPE)
endfunction()

# sce_codegen_generation_request(DOCUMENT <document> OUTPUT_DIR <dir>
#     LANGUAGE <cpp|c11> [STEM <stem>] [DEPLOY <deploy.yaml>]
#     [INPUT_ROOT <dir>] [CHILDREN_HELD_IN <dir>] [CHILD_GENERATIONS]
#     [CLAIM <global property>] [ARGS <arg>...] [COMMENT <text>])
#
#   Ask ahead for the plan a later `sce_codegen_generation(<PREFIX> ...)`
#   with the same options asks (`sce_codegen_plan_request`), so that the
#   generations a caller declares one after another are planned in one batch
#   (`sce_codegen_plan_prefetch`) before the first of them. COMMENT is taken
#   so a caller can pass the very options it declares with; it changes no
#   request.
function(sce_codegen_generation_request)
    cmake_parse_arguments(PARSE_ARGV 0 _SCGR "CHILD_GENERATIONS"
        "STEM;DOCUMENT;OUTPUT_DIR;LANGUAGE;DEPLOY;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM;COMMENT"
        "ARGS")
    if(_SCGR_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_generation_request: unexpected argument(s): "
            "${_SCGR_UNPARSED_ARGUMENTS}")
    endif()
    _sce_codegen_generation_plan_options(_OPTIONS _SCGR "sce_codegen_generation_request")
    sce_codegen_plan_request(${_OPTIONS})
endfunction()

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
        _sce_codegen_generation_plan_options(_PLAN_OPTIONS _SCGA "sce_codegen_generation(${PREFIX})")
        if(_SCGA_DEPLOY)
            list(APPEND _DEPENDS "${_SCGA_DEPLOY}")
        endif()
        sce_codegen_plan(_SCGA_PLAN ${_PLAN_OPTIONS})
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
