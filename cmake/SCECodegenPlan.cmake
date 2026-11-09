# SCE Codegen Plan
#
# Which files one `sce-codegen generate` run writes, asked of the generator.
#
# A build system has to declare a command's outputs before the command has
# run, and the files a document produces are not all files its author wrote:
# an inline `<invoke>` becomes a child document with a machine of its own
# (`<machine>__sce_synth_invoke__<id>`), an `<invoke>` that names its target
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
# the test trees declare every generation they run through
# `sce_codegen_generation`, which asks it and declares the command, and
# `the_conformance_cmake_route_declares_what_the_generator_plans` holds all
# three to that.
#
# The machine a statechart's generation writes is named by the generator too:
# its machine name is the document's file stem written as an identifier
# (docs/SCE_ACCEPTED_SUBSET.md, `job-tester.scxml` is the machine `job_tester`,
# written to `job_tester_sm.h` in `namespace SCE::Generated::job_tester`), and
# every plan states it (`--plan-machines`), the run's own and each child
# generation's. The module names a machine's files from that statement and
# never from the stem: a stem it spelled into `<stem>_sm.h` itself was a
# second spelling of the generator's name, and a hyphenated or camelCase stem
# made the plan refuse the generation it was declaring, as one that does not
# name the file its command produces.
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
# `sce-codegen --reads`, the prerequisites the depfile of the same run
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
# answer where the next ask finds it. Every plan the tree declares is asked
# before any is declared ("request first, declare after"):
#   - when this module is first included, every answer the last configure of
#     this build tree asked for that is no longer current is asked again in
#     one batch, which is every plan after the generator is rebuilt
#     (`_sce_codegen_plan_refresh`);
#   - a route that declares its own commands from `sce_codegen_plan` queues
#     the plans of its fixtures (`sce_codegen_plan_request`) and sends them
#     together (`sce_codegen_plan_prefetch`) before declaring them: the
#     conformance route for every fixture the registry lists, and, after
#     their last registration, the C11 conformance and C++ integration
#     fixtures; the C11 integration fixtures, whose targets compile each
#     one's files right after it is registered, declare at the end of their
#     directory (`cmake_language(DEFER)`);
#   - every other generation is a call of `sce_codegen_generation`, which
#     queues its plan when it is called and declares nothing then: every
#     generation called in a directory is declared when that directory ends
#     (`_sce_codegen_generations_declare`), after every plan queued by then
#     was sent in one batch, and the call hands its caller generator
#     expressions for the files a target compiles;
#   - a plan asked otherwise and not kept is sent alone, when it is asked.
#     The tree asks none so, and is held to it: a plan in any CMake file of
#     the tree that no request of the same file asks ahead, word for word,
#     is refused by
#     `the_conformance_cmake_route_declares_what_the_generator_plans`.
# At the end of the configure, the answers no plan asked for are removed,
# the list of the asked ones is kept for the next configure, and one line
# says how many plans were asked, how many were kept, how many the generator
# was sent and how many of those were sent alone (`_sce_codegen_plan_settle`),
# followed, when a kept answer was not current, by why (`stale.txt` holds
# every reason).
#
# Why `sce_codegen_generation` defers its own declaration rather than each
# caller asking ahead and declaring after: a request and a declaration
# written at the call would be the one generation's options written twice,
# two lists to keep word for word in step, and a caller that wrote only the
# second would send its plan alone with nothing to say so. Made by the one
# call, the request and the declaration are read from the one list the call
# was given. What a caller gives up is the plan's answer when the call
# returns, and no caller needs it then: a target takes the files as a
# generator expression, which the declaration fills before the build system
# is generated, and the child generations a plan holds
# (`CHILD_GENERATIONS`) are declared by the declaration itself rather than
# by the caller reading the plan.
#
# What the cache cannot see is what no list names, and a file put back with
# an older date than the answer. A depfile has the same blind spots;
# removing `<build>/sce-codegen-plans` asks every plan again.
#
# ── Three steps a build compiles the files of ──────────────────────────
#
# A generation is one of the three commands whose files a target compiles,
# and each is planned and declared the same way: one document (`generate`,
# DOCUMENT), a document set compiled together (`orchestrate`, DOCUMENTS: a
# broadcast log is sized from its element's document, which only the set's
# compile reads), and a conformance harness rendered from a fixture catalog
# (`generate-conformance`, MANIFEST). The generator answers `--plan` for each,
# writes a depfile for each, and lists what each plan read. A set's or a
# harness's step produces the first file its plan names, as a forge
# document's does, and the rest are its byproducts.
#
# ── Who includes this ───────────────────────────────────────────────────
#
# The tree's routes and test directories; the projects configured on their
# own (`cmake -S backends/<c|cpp>/forge-runtime/tests/<round_trip|
# conformance>`), which declare every generation through
# `sce_codegen_generation` as the tree does; and `SCECodegen.cmake`, whose
# public functions (`sce_add_state_machine` and the rest) are declared
# through it too, so a consumer's build gets each generation's files from the
# plan, one producer per file, its depfile, its children and what its
# machines require. It is installed beside `SCECodegen.cmake` for that
# (`SCE_CODEGEN_CMAKE_FILES`, sce/sce_codegen_assets.cmake).

include_guard(GLOBAL)

# `cmake_language(DEFER)` is 3.19; a DEPFILE under the Makefile generators is
# 3.20 (the top CMakeLists.txt of SCE has the measurements).
if(CMAKE_VERSION VERSION_LESS 3.20)
    message(FATAL_ERROR
        "SCECodegenPlan.cmake declares each generation with a DEPFILE and at the end of "
        "its directory, which needs CMake 3.20 or newer; this is CMake ${CMAKE_VERSION}")
endif()

# The policies the functions below are run with, recorded where they are
# defined: an includer's own `cmake_minimum_required` may be older. A DEPFILE
# reaches Ninja with targets that match the command's outputs (CMP0116), and
# `if(... IN_LIST ...)` is a condition (CMP0057). An `include()` pushes a
# policy scope of its own, so neither leaks into the includer.
cmake_policy(SET CMP0057 NEW)
cmake_policy(SET CMP0116 NEW)

# The generator: the one an includer resolved, or the one
# `SCEFindCodegen.cmake` resolves as every harness does, which refuses with the
# command that builds it when there is none. A cached path whose file is gone
# is resolved again there.
if(NOT SCE_CODEGEN OR NOT EXISTS "${SCE_CODEGEN}")
    include(${CMAKE_CURRENT_LIST_DIR}/SCEFindCodegen.cmake)
endif()

# The environment every run of the generator this module makes is given, a
# plan or a generation alike, and which a kept answer is keyed on
# (`_sce_codegen_plan_entry`): the templates it renders with, when the build
# names them (`SCE_TEMPLATE_DIR`, which `SCECodegen.cmake` finds beside an
# installed or vendored generator, whose own fallback finds none). Read once,
# when the module is first included, so a plan asked ahead and the plan it
# declares are asked with the one environment whatever scope each is called
# from.
set_property(GLOBAL PROPERTY _SCE_CODEGEN_ENVIRONMENT "")
if(SCE_TEMPLATE_DIR)
    get_filename_component(_SCE_CODEGEN_TEMPLATES "${SCE_TEMPLATE_DIR}" ABSOLUTE)
    set_property(GLOBAL PROPERTY _SCE_CODEGEN_ENVIRONMENT
        "SCE_TEMPLATE_DIR=${_SCE_CODEGEN_TEMPLATES}")
    unset(_SCE_CODEGEN_TEMPLATES)
endif()

# _sce_codegen_launcher(<VAR>)
#
#   What a command that runs the generator is prefixed with: nothing, or
#   `cmake -E env` and the environment the module gives the generator.
function(_sce_codegen_launcher VAR)
    get_property(_GIVEN GLOBAL PROPERTY _SCE_CODEGEN_ENVIRONMENT)
    if(_GIVEN)
        set(${VAR} "${CMAKE_COMMAND}" -E env ${_GIVEN} PARENT_SCOPE)
    else()
        set(${VAR} "" PARENT_SCOPE)
    endif()
endfunction()

# sce_codegen_write_input(<path> <content>)
#
#   Write an input of the build that is composed when the tree is configured
#   (a document, a deploy or a catalog a plan reads; a source a target
#   compiles), and leave the file as it is, date included, when it already
#   holds <content>.
#
#   A kept plan is reused only while no file it read is newer than it
#   (`_sce_codegen_plan_fresh`), and a depfile re-runs its generation when one
#   is. A `file(WRITE)` of the same bytes on every configure dates the file
#   anew each time, so every configure sent its plan to the generator again
#   and every build after it generated again: a reconfigure that changed
#   nothing was not a no-op. Compared here, in the configure: a
#   `configure_file` from a staging copy reads a file written on every
#   configure as its input, and `file(CONFIGURE)` and `file(GENERATE)` write
#   after the plans were asked. A file of the build composed at configure time
#   is written through this in every CMake file that declares a generation,
#   and `the_conformance_cmake_route_declares_what_the_generator_plans` refuses
#   a `file(WRITE)` there.
function(sce_codegen_write_input PATH CONTENT)
    if(EXISTS "${PATH}")
        file(READ "${PATH}" _KEPT)
        if("${_KEPT}" STREQUAL "${CONTENT}")
            return()
        endif()
    endif()
    file(WRITE "${PATH}" "${CONTENT}")
endfunction()

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

# _sce_codegen_plan_run(<RUN_VAR> <FORMATS_VAR> <PARSED> <CALLER>)
#
#   The command a generation is run with and what it reads, from the options
#   `cmake_parse_arguments` parsed into `<PARSED>_*` in the caller's scope:
#   exactly one of
#     DOCUMENT  <document>      `generate <document>`
#     DOCUMENTS <document>...   `orchestrate --document <document>...`, a set
#                               compiled together
#     MANIFEST  <catalog>       `generate-conformance --manifest <catalog>`, a
#                               conformance harness
#   and whether that command formats what it writes (`--no-format`,
#   `--format-style`): a harness is C or C++ as its templates write it. The
#   options of one document's generation alone (STEM, CHILDREN_HELD_IN,
#   CHILD_GENERATIONS) are refused with the other two, which have no child to
#   hold, and an input root with a harness, whose source set is its catalog's.
#   CALLER names the call in a refusal.
function(_sce_codegen_plan_run RUN_VAR FORMATS_VAR PARSED CALLER)
    set(_FORMS "")
    foreach(_FORM DOCUMENT DOCUMENTS MANIFEST)
        if(NOT "${${PARSED}_${_FORM}}" STREQUAL "")
            list(APPEND _FORMS ${_FORM})
        endif()
    endforeach()
    list(LENGTH _FORMS _COUNT)
    if(NOT _COUNT EQUAL 1)
        message(FATAL_ERROR
            "${CALLER}: a generation reads one DOCUMENT, a set of DOCUMENTS or the MANIFEST "
            "of a harness, exactly one of them (given: ${_FORMS})")
    endif()
    set(_FORMATS TRUE)
    if(_FORMS STREQUAL "DOCUMENT")
        set(_RUN generate "${${PARSED}_DOCUMENT}")
    else()
        foreach(_ONE_DOCUMENTS STEM CHILDREN_HELD_IN CHILD_GENERATIONS)
            if(${PARSED}_${_ONE_DOCUMENTS})
                message(FATAL_ERROR
                    "${CALLER}: ${_ONE_DOCUMENTS} is one document's generation's; a set "
                    "and a harness name their files themselves and start no child")
            endif()
        endforeach()
        if(_FORMS STREQUAL "DOCUMENTS")
            set(_RUN orchestrate)
            foreach(_MEMBER IN LISTS ${PARSED}_DOCUMENTS)
                list(APPEND _RUN --document "${_MEMBER}")
            endforeach()
        else()
            if(NOT "${${PARSED}_INPUT_ROOT}" STREQUAL "")
                message(FATAL_ERROR
                    "${CALLER}: a harness takes no INPUT_ROOT; its source set is the "
                    "resources beside its MANIFEST")
            endif()
            set(_RUN generate-conformance --manifest "${${PARSED}_MANIFEST}")
            set(_FORMATS FALSE)
        endif()
    endif()
    set(${RUN_VAR} "${_RUN}" PARENT_SCOPE)
    set(${FORMATS_VAR} "${_FORMATS}" PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_arguments(<VAR> <run> <formats> <language> <output dir>
#     <input root> <child generations> [<argument>...])
#
#   The arguments `sce-codegen` is run with to plan one generation, after the
#   program: the one spelling of a plan's request, which `sce_codegen_plan`
#   asks and `sce_codegen_plan_request` asks ahead. <run> is the command and
#   what it reads (`_sce_codegen_plan_run`), <formats> whether it takes a
#   formatting flag. An empty input root passes none; a true <child
#   generations> asks for the plans of the children too (`--plan-children`).
#
#   `--plan-requires`: every plan states what the machines of each of its
#   generations need linked (`<PREFIX>_REQUIRES`), so a route that links a
#   machine reads it from the answer it already asks for.
#
#   `--plan-machines`: every plan states the machine name of the statechart
#   each of its generations writes (`<PREFIX>_MACHINE_NAME`), which names the
#   machine's files (`_sce_codegen_plan_machine`): the generator's name, read
#   rather than spelled again here from the stem.
#
#   `--no-format`: formatting changes bytes and never which files, and a plan
#   is asked once per generation when the tree is configured, so it neither
#   waits for clang-format on every file nor needs one there.
function(_sce_codegen_plan_arguments VAR RUN FORMATS LANGUAGE OUTPUT_DIR INPUT_ROOT CHILD_GENERATIONS)
    set(_ARGUMENTS --plan --plan-requires --plan-machines)
    if(CHILD_GENERATIONS)
        list(APPEND _ARGUMENTS --plan-children)
    endif()
    list(APPEND _ARGUMENTS ${RUN} -l "${LANGUAGE}" -o "${OUTPUT_DIR}")
    if(INPUT_ROOT)
        list(APPEND _ARGUMENTS --input-root "${INPUT_ROOT}")
    endif()
    if(FORMATS)
        list(APPEND _ARGUMENTS --no-format)
    endif()
    list(APPEND _ARGUMENTS ${ARGN})
    set(${VAR} "${_ARGUMENTS}" PARENT_SCOPE)
endfunction()

# _sce_codegen_plan_options_judged(<PARSED> <CALLER>)
#
#   Refuse the options parsed into `<PARSED>_*` that no plan can be asked
#   with: an output directory and a language are always given, a document's
#   generation names its STEM, and the children of a generation are planned
#   only in a language whose machine this module names
#   (`_sce_codegen_plan_names_machines`), since a child's command waits for
#   the machine of the generation that writes its document.
function(_sce_codegen_plan_options_judged PARSED CALLER)
    foreach(_REQUIRED OUTPUT_DIR LANGUAGE)
        if("${${PARSED}_${_REQUIRED}}" STREQUAL "")
            message(FATAL_ERROR "${CALLER}: ${_REQUIRED} is required")
        endif()
    endforeach()
    if(NOT "${${PARSED}_DOCUMENT}" STREQUAL "" AND "${${PARSED}_STEM}" STREQUAL "")
        message(FATAL_ERROR "${CALLER}: STEM is required with DOCUMENT")
    endif()
    if(${PARSED}_CHILD_GENERATIONS)
        _sce_codegen_plan_names_machines(_NAMES_MACHINES "${${PARSED}_LANGUAGE}")
        if(NOT _NAMES_MACHINES)
            message(FATAL_ERROR
                "${CALLER}: CHILD_GENERATIONS plans the children of a ${${PARSED}_LANGUAGE} "
                "generation, a language whose machine this module does not name; the "
                "languages it names are cpp and c11")
        endif()
    endif()
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
#   list to the generator's sources. A variable the module gives the generator
#   itself (`_SCE_CODEGEN_ENVIRONMENT`) is keyed on the value it gives.
function(_sce_codegen_plan_entry ENTRY_VAR LINE_VAR)
    foreach(_ARGUMENT IN LISTS ARGN)
        if("${_ARGUMENT}" MATCHES "[\t\n]")
            message(FATAL_ERROR
                "sce_codegen_plan: the argument \"${_ARGUMENT}\" holds a tab or a line "
                "break, which a request to sce-codegen plan-batch cannot carry")
        endif()
    endforeach()
    set(_ENVIRONMENT SCE_FILE_ROOT SCE_SCHEMAS_DIR SCE_TEMPLATE_DIR SCE_WORKSPACE_ROOT)
    get_property(_GIVEN GLOBAL PROPERTY _SCE_CODEGEN_ENVIRONMENT)
    set(_IDENTITY "${SCE_CODEGEN}")
    foreach(_NAME IN LISTS _ENVIRONMENT)
        set(_VALUE "$ENV{${_NAME}}")
        foreach(_ASSIGNMENT IN LISTS _GIVEN)
            if("${_ASSIGNMENT}" MATCHES "^${_NAME}=(.*)$")
                set(_VALUE "${CMAKE_MATCH_1}")
            endif()
        endforeach()
        string(APPEND _IDENTITY "\n${_NAME}=${_VALUE}")
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
            # `!<tool>`: a tool the run shaped bytes with (the C++ formatter).
            # A plan answers which files a generation writes, not their bytes,
            # so the tool's version moves no answer kept here.
            if("${_READ}" MATCHES "^!")
                continue()
            endif()
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
            # The document a plan generates, the first of a set, or the
            # catalog of a harness (`_sce_codegen_plan_arguments`).
            file(READ "${ENTRY}.request" _ASKED)
            if("${_ASKED}" MATCHES "\t(generate|--document|--manifest)\t([^\t\n]*)")
                set(_DOCUMENT "${CMAKE_MATCH_2}")
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
    # Each plan of the batch is a process of its own, which inherits the
    # environment the batch is run with.
    _sce_codegen_launcher(_LAUNCH)
    execute_process(
        COMMAND ${_LAUNCH} "${SCE_CODEGEN}" plan-batch
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
        # Asked ahead by no request, or asked ahead and answered by no batch:
        # one process for one plan, which the settle line counts.
        _sce_codegen_plan_count(ALONE 1)
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
    foreach(_COUNTER ASKED KEPT SENT BATCHES ALONE)
        get_property(_COUNT_${_COUNTER} GLOBAL PROPERTY "_SCE_CODEGEN_PLAN_${_COUNTER}")
        if("${_COUNT_${_COUNTER}}" STREQUAL "")
            set(_COUNT_${_COUNTER} 0)
        endif()
    endforeach()
    # ALONE is how many of the SENT went one to a batch because no batch had
    # answered them when they were asked: each is a process the configure
    # waited for by itself.
    message(STATUS
        "SCE: generation plans: ${_COUNT_ASKED} asked, ${_COUNT_KEPT} answered from the "
        "cache; ${_COUNT_SENT} sent to sce-codegen in ${_COUNT_BATCHES} batch(es), "
        "${_COUNT_ALONE} of them alone (${_CACHE})")

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

# sce_codegen_plan_request(
#     STEM <stem> DOCUMENT <document> | DOCUMENTS <document>... | MANIFEST <catalog>
#     OUTPUT_DIR <dir> LANGUAGE <language>
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
        "STEM;DOCUMENT;MANIFEST;OUTPUT_DIR;LANGUAGE;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM"
        "DOCUMENTS;ARGS")
    if(_SCR_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_plan_request: unexpected argument(s): ${_SCR_UNPARSED_ARGUMENTS}")
    endif()
    # Judged as `sce_codegen_plan` judges them.
    _sce_codegen_plan_options_judged(_SCR "sce_codegen_plan_request")
    _sce_codegen_plan_run(_RUN _FORMATS _SCR "sce_codegen_plan_request")
    _sce_codegen_plan_arguments(_REQUEST "${_RUN}" "${_FORMATS}" "${_SCR_LANGUAGE}"
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

# _sce_codegen_plan_settle_last()
#
#   Schedule the settle again, after every call deferred to the end of the
#   top-level directory so far. Deferred calls run in the order they were
#   scheduled, and the settle was scheduled when this module was included: a
#   declaration deferred to the end of the top-level directory after that
#   would ask its plans after the settle had written the list of the asked
#   ones and removed every answer not on it.
function(_sce_codegen_plan_settle_last)
    get_property(_CALL GLOBAL PROPERTY _SCE_CODEGEN_PLAN_SETTLE_CALL)
    cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}" CANCEL_CALL "${_CALL}")
    cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}" ID_VAR _CALL
        CALL _sce_codegen_plan_settle)
    set_property(GLOBAL PROPERTY _SCE_CODEGEN_PLAN_SETTLE_CALL "${_CALL}")
endfunction()

# Once per configure, before any plan is asked: bring up to date what the last
# configure asked for, and settle the cache when the configure ends. A script
# (`cmake -P`) has no configure to end.
if(NOT CMAKE_SCRIPT_MODE_FILE)
    _sce_codegen_plan_refresh()
    cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}" ID_VAR _SCE_CODEGEN_PLAN_SETTLE_CALL
        CALL _sce_codegen_plan_settle)
    set_property(GLOBAL PROPERTY _SCE_CODEGEN_PLAN_SETTLE_CALL "${_SCE_CODEGEN_PLAN_SETTLE_CALL}")
    unset(_SCE_CODEGEN_PLAN_SETTLE_CALL)
endif()

# _sce_codegen_plan_names_machines(<VAR> <LANGUAGE>)
#
#   Whether this module names the machine a generation of LANGUAGE writes:
#   whether `_sce_codegen_plan_machine` gives a machine of that language any
#   file, asked of it so the languages are listed in the one place (C++ and
#   C11, the languages a target compiles at build time).
function(_sce_codegen_plan_names_machines VAR LANGUAGE)
    _sce_codegen_plan_machine(_FILES _UNUSED_DEPFILE "machine" "${LANGUAGE}")
    if("${_FILES}" STREQUAL "")
        set(${VAR} FALSE PARENT_SCOPE)
    else()
        set(${VAR} TRUE PARENT_SCOPE)
    endif()
endfunction()

# _sce_codegen_plan_machine(<MACHINE_VAR> <DEPFILE_VAR> <MACHINE_NAME> <LANGUAGE>)
#
#   The files of the machine MACHINE_NAME a generation of each language
#   writes, header first, and the name of the depfile it is asked to write
#   beside it. The one place a build names them: every declaration of a
#   machine reads them from here, and every plan is held to naming them.
#
#   MACHINE_NAME is the generator's: the name a plan states for the
#   generation (`\tmachine\t<generation>\t<name>`, `--plan-machines`), the
#   document's file stem written as an identifier, never the stem as the
#   caller wrote it. `job-tester.scxml` is the machine `job_tester`, whose
#   header is `job_tester_sm.h`.
#
#   A language a target compiles at build time is C++ or C11; a generation
#   of another one (Rust, Kotlin, Go, Python, which `sce_add_state_machine`
#   generates for a build of their own) has no machine named here, both are
#   empty, and its step produces the first file its plan names, as a forge
#   document's does. So is a generation whose plan states no machine name.
function(_sce_codegen_plan_machine MACHINE_VAR DEPFILE_VAR MACHINE_NAME LANGUAGE)
    if("${MACHINE_NAME}" STREQUAL "")
        set(${MACHINE_VAR} "" PARENT_SCOPE)
        set(${DEPFILE_VAR} "" PARENT_SCOPE)
    elseif(LANGUAGE STREQUAL "cpp")
        set(${MACHINE_VAR} "${MACHINE_NAME}_sm.h" PARENT_SCOPE)
        set(${DEPFILE_VAR} "${MACHINE_NAME}_sm.h.d" PARENT_SCOPE)
    elseif(LANGUAGE STREQUAL "c11")
        # The source is the second output and the depfile sits beside it. With
        # the source first, ninja on CMake 3.x read the depfile as describing
        # another edge ("expected depfile ... to mention '_sm.c', got '_sm.h'")
        # and regenerated every C11 machine on every build.
        set(${MACHINE_VAR} "${MACHINE_NAME}_sm.h" "${MACHINE_NAME}_sm.c" PARENT_SCOPE)
        set(${DEPFILE_VAR} "${MACHINE_NAME}_sm.c.d" PARENT_SCOPE)
    else()
        set(${MACHINE_VAR} "" PARENT_SCOPE)
        set(${DEPFILE_VAR} "" PARENT_SCOPE)
    endif()
endfunction()

# sce_codegen_plan(<PREFIX>
#     STEM <stem> DOCUMENT <document> | DOCUMENTS <document>... | MANIFEST <catalog>
#     OUTPUT_DIR <dir> LANGUAGE <language>
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [CHILD_GENERATIONS]
#     [CLAIM <global property>]
#     [ARGS <arg>...])
#
#   Ask the generator which files generating DOCUMENT (or compiling the set
#   DOCUMENTS, or rendering the harness of MANIFEST) into OUTPUT_DIR writes.
#
#   STEM        The document's own stem, as its file writes it, which names
#               the generation in what the module says and is its name among
#               the generations a plan holds. It does not name the machine's
#               files: the generator names the machine (the stem written as
#               an identifier) and the plan states it (`<PREFIX>_MACHINE_NAME`).
#               A document of a forge kind names its files itself, and its
#               plan is read for them.
#   DOCUMENT    A document on disk now, or the library name of a standard
#               statechart (`sce:std/...`), which the generator reads from the
#               library it embeds.
#   DOCUMENTS   A document set compiled together (`orchestrate`), each a
#               document on disk or a library name. The step produces the
#               first file the plan names.
#   MANIFEST    A forge fixture catalog, whose conformance harness
#               (`generate-conformance`) is the one file the step produces.
#   OUTPUT_DIR  Where the run writes. Every file of the answer has to land
#               directly in it: this declares the outputs of a flat directory.
#   LANGUAGE    The language generated: `cpp` and `c11`, which a target
#               compiles, have the machine of a statechart named here
#               (`_sce_codegen_plan_machine`, from the name the plan states);
#               another language's step produces the first file its plan
#               names.
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
#     <PREFIX>_RUN            the command the run is and what it reads:
#                             `generate <document>`, `orchestrate --document
#                             <document>...` or `generate-conformance
#                             --manifest <catalog>`; for a plan FROM another,
#                             `generate` and the child document
#     <PREFIX>_FORMATS        TRUE when that command takes a formatting flag
#                             (a harness does not)
#     <PREFIX>_DOCUMENT       the document the run reads: DOCUMENT, or for a
#                             plan FROM another, the child document; empty
#                             for a set and a harness
#     <PREFIX>_DEPENDS        what the command depends on before its depfile
#                             is written: the document (unless it is a
#                             library name) and each document staged from
#                             CHILDREN_HELD_IN, each document of a set that
#                             is not a library name, or a harness's catalog;
#                             or for a plan FROM another, as above
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
#     <PREFIX>_MACHINE_NAME   the machine name the generator states for the
#                             statechart whose machine the run writes
#                             (`--plan-machines`): the document's file stem
#                             written as an identifier, `job_tester` for
#                             `job-tester.scxml`, which names the machine's
#                             files, its namespace and its types; or for a
#                             plan FROM another, the one it states for the
#                             child's generation. Empty for a run that writes
#                             no statechart's machine (a forge document, a
#                             set, a harness)
#     <PREFIX>_OUTPUTS        the paths of the machine, header first: the
#                             command's OUTPUT, and the header is the
#                             depfile's one target, which ninja needs to be
#                             the edge's first output. A document of a forge
#                             kind writes no machine: its OUTPUT is the first
#                             file it writes, its primary artefact and the
#                             depfile's target, and the depfile is named
#                             after it
#     <PREFIX>_BYPRODUCTS     the paths of every other file of <PREFIX>_FILES
#                             no plan of CLAIM declared before
#     <PREFIX>_BYPRODUCTS_ARG `BYPRODUCTS` followed by them, or nothing when
#                             there are none
#     <PREFIX>_DEPFILE        the depfile the run is asked to write with
#                             `--write-deps`, and the command's DEPFILE
#     <PREFIX>_HEADERS        the paths of every header the run writes
#     <PREFIX>_SOURCES        the paths of every C translation unit it writes
#     <PREFIX>_REQUIRES       what the machines the run writes need linked
#                             beside them, as the generator states it
#                             (`--plan-requires`): each of `lua`, `dom`, `http`
#                             and `clock` the backend decided while rendering
#                             them, or for a plan FROM another, what that plan
#                             states for the child's generation. Empty for a
#                             backend that states none. The plan also records
#                             it against every file the run writes (the GLOBAL
#                             property `SCE_GENERATED_REQUIRES:<path>`), so a
#                             target that compiles a generated file links what
#                             its generation requires (`sce_c_link_runtime`,
#                             backends/c/runtime/CMakeLists.txt) whichever
#                             route declared it
#     <PREFIX>_DECLARED_ELSEWHERE
#                             TRUE when a plan of CLAIM declared this machine
#                             before, so its command is that plan's
#
# `sce_sourcemap.json` is left out of the answer. Every document generated into
# one directory writes it, to the one name, so no command can own it: two
# commands declaring one file as an output is an error of its own.
function(sce_codegen_plan PREFIX)
    cmake_parse_arguments(PARSE_ARGV 1 _SCP "CHILD_GENERATIONS"
        "STEM;DOCUMENT;MANIFEST;OUTPUT_DIR;LANGUAGE;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM;FROM;CHILD"
        "DOCUMENTS;ARGS")
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
    set(_REQUIRES "")
    # The machine name the generator states for the run's statechart, empty
    # for a run that writes none.
    set(_MACHINE_NAME "")
    # Whether the run is one document's generation, whose machine the plan
    # names; a set's and a harness's step produces the first file it writes.
    set(_ONE_DOCUMENT TRUE)

    if(_SCP_FROM)
        # ── A generation another plan holds ─────────────────────────────
        foreach(_GIVEN STEM DOCUMENT DOCUMENTS MANIFEST OUTPUT_DIR LANGUAGE INPUT_ROOT
                CHILDREN_HELD_IN CLAIM ARGS)
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
        set(_RUN generate "${_DOCUMENT}")
        set(_FORMATS TRUE)
        set(_ARGS "${${_FROM}_ARGS}")
        set(_REQUIRES "${${_FROM}_REQUIRES}")
        set(_MACHINE_NAME "${${_FROM}_MACHINE_NAME}")
        if("${_MACHINE_NAME}" STREQUAL "")
            # A child generation is one of a language whose machine this
            # module names (`_sce_codegen_plan_options_judged`), and its
            # command produces the child's machine.
            message(FATAL_ERROR
                "sce_codegen_plan(${PREFIX} FROM ${_SCP_FROM} CHILD ${_SCP_CHILD}): the plan "
                "states no machine name for the generation of ${_SCP_CHILD} "
                "(`\\tmachine\\t<generation>\\t<name>`, --plan-machines), so the machine "
                "its command produces cannot be declared")
        endif()
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
        _sce_codegen_plan_options_judged(_SCP "sce_codegen_plan(${PREFIX})")
        _sce_codegen_plan_run(_RUN _FORMATS _SCP "sce_codegen_plan(${PREFIX})")
        set(_STEM "${_SCP_STEM}")
        set(_OUTPUT_DIR "${_SCP_OUTPUT_DIR}")
        set(_LANGUAGE "${_SCP_LANGUAGE}")
        set(_CLAIM "${_SCP_CLAIM}")
        set(_ARGS "${_SCP_ARGS}")
        set(_INPUT_ROOT "${_SCP_INPUT_ROOT}")

        if(NOT "${_SCP_DOCUMENT}" STREQUAL "")
            set(_DOCUMENT "${_SCP_DOCUMENT}")
            if(NOT _SCP_INPUT_ROOT AND NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
                # The root the generator takes when it is given none: the
                # document's own directory, which a child's generation,
                # reading its document from the output directory, is given
                # explicitly.
                get_filename_component(_INPUT_ROOT "${_SCP_DOCUMENT}" DIRECTORY)
            endif()
            if(NOT "${_SCP_DOCUMENT}" MATCHES "^sce:")
                set(_DEPENDS "${_SCP_DOCUMENT}")
            endif()
        else()
            set(_ONE_DOCUMENT FALSE)
            # What a set reads before its depfile is written: each document
            # of it on disk (a library name is in the generator); a
            # harness's, its catalog.
            foreach(_MEMBER IN LISTS _SCP_DOCUMENTS _SCP_MANIFEST)
                if(NOT "${_MEMBER}" MATCHES "^sce:")
                    list(APPEND _DEPENDS "${_MEMBER}")
                endif()
            endforeach()
        endif()
        # The answer kept for this request while it is current, the
        # generator's otherwise (`_sce_codegen_plan_answer`).
        _sce_codegen_plan_arguments(_REQUEST "${_RUN}" "${_FORMATS}" "${_LANGUAGE}"
            "${_OUTPUT_DIR}" "${_SCP_INPUT_ROOT}" "${_SCP_CHILD_GENERATIONS}" ${_SCP_ARGS})
        _sce_codegen_plan_answer(_PLAN_RESULT _PLAN_OUTPUT _PLAN_ERROR ${_REQUEST})
        if(NOT _PLAN_RESULT EQUAL 0)
            message(FATAL_ERROR
                "sce-codegen --plan ${_RUN} failed (${_LANGUAGE}), so the outputs of its "
                "generation cannot be declared. A sce-codegen older than --plan (or, asked "
                "with CHILD_GENERATIONS, than --plan-children, or than --reads, or "
                "asked of a set or a harness, than their plans) answers with a usage error: "
                "rebuild it from this tree.\n${_PLAN_ERROR}")
        endif()

        # A line that begins with a tab names no file: it states something of
        # a generation, the run's own when <generation> is empty, either the
        # name of the machine it writes, `\tmachine\t<generation>\t<name>`
        # (`--plan-machines`), or what its machines need linked,
        # `\trequires\t<generation>\t<requirement>` (`--plan-requires`).
        # Every other line with a tab is a file of a child document's
        # generation, `<child document>\t<file>`; every other line is the
        # run's own.
        set(_GENERATION_LINES "")
        set(_STATEMENT_LINES "")
        string(REPLACE "\n" ";" _PLANNED "${_PLAN_OUTPUT}")
        foreach(_LINE ${_PLANNED})
            string(FIND "${_LINE}" "\t" _TAB)
            if(_TAB EQUAL 0)
                list(APPEND _STATEMENT_LINES "${_LINE}")
                continue()
            endif()
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
                    "sce-codegen --plan ${_RUN} (${_LANGUAGE}) names ${_PATH}, which is not "
                    "directly in ${_OUTPUT_DIR}: sce_codegen_plan declares the outputs of a "
                    "flat output directory")
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

        # The name of the machine each generation writes and what its machines
        # need linked: the run's own, and each child generation's, which a
        # plan FROM this one reads.
        foreach(_CHILD ${_GENERATIONS})
            set(_GENERATION_${_CHILD}_REQUIRES "")
            set(_GENERATION_${_CHILD}_MACHINE_NAME "")
        endforeach()
        foreach(_LINE ${_STATEMENT_LINES})
            if(NOT "${_LINE}" MATCHES "^\t(machine|requires)\t([^\t]*)\t([^\t]+)$")
                message(FATAL_ERROR
                    "sce-codegen --plan ${_RUN} (${_LANGUAGE}) answers "
                    "a line this module cannot read: \"${_LINE}\". A line that begins with a "
                    "tab states the machine a generation writes, "
                    "`\\tmachine\\t<generation>\\t<name>`, or what it requires, "
                    "`\\trequires\\t<generation>\\t<requirement>`.")
            endif()
            set(_STATED "${CMAKE_MATCH_1}")
            set(_VALUE "${CMAKE_MATCH_3}")
            if("${CMAKE_MATCH_2}" STREQUAL "")
                if(_STATED STREQUAL "requires")
                    list(APPEND _REQUIRES "${_VALUE}")
                else()
                    set(_MACHINE_NAME "${_VALUE}")
                endif()
                continue()
            endif()
            get_filename_component(_CHILD_NAME "${CMAKE_MATCH_2}" NAME)
            string(REGEX REPLACE "\\.scxml$" "" _CHILD "${_CHILD_NAME}")
            if(NOT "${_CHILD}" IN_LIST _GENERATIONS)
                message(FATAL_ERROR
                    "sce-codegen --plan generate for ${_SCP_DOCUMENT} (${_LANGUAGE}) states "
                    "that the generation of ${_CHILD_NAME} ${_STATED} ${_VALUE}, and "
                    "plans no generation of it")
            endif()
            if(_STATED STREQUAL "requires")
                list(APPEND _GENERATION_${_CHILD}_REQUIRES "${_VALUE}")
            else()
                set(_GENERATION_${_CHILD}_MACHINE_NAME "${_VALUE}")
            endif()
        endforeach()
        # Which generation writes each child document, and the name of the
        # machine it writes, whose header a child's command waits for: the run
        # first, then each generation in the order it was planned.
        set(_WRITERS_OF "${_STEM}")
        set(_WRITER_FILES_${_STEM} ${_FILES})
        set(_WRITER_MACHINE_NAME_${_STEM} "${_MACHINE_NAME}")
        foreach(_CHILD ${_GENERATIONS})
            set(_WRITER "")
            # A document staged from CHILDREN_HELD_IN is there before the build
            # starts: no generation writes it, and its command waits for none.
            if("${_OUTPUT_DIR}/${_CHILD}.scxml" IN_LIST _STAGED)
                set(_GENERATION_${_CHILD}_WRITER "")
                list(APPEND _WRITERS_OF "${_CHILD}")
                set(_WRITER_FILES_${_CHILD} ${_GENERATION_${_CHILD}_FILES})
                set(_WRITER_MACHINE_NAME_${_CHILD} "${_GENERATION_${_CHILD}_MACHINE_NAME}")
                continue()
            endif()
            foreach(_CANDIDATE ${_WRITERS_OF})
                if("${_CHILD}.scxml" IN_LIST _WRITER_FILES_${_CANDIDATE})
                    if("${_WRITER_MACHINE_NAME_${_CANDIDATE}}" STREQUAL "")
                        message(FATAL_ERROR
                            "sce-codegen --plan-children for ${_SCP_DOCUMENT} (${_LANGUAGE}) "
                            "plans the generation of ${_CHILD}.scxml, whose document the "
                            "generation of ${_CANDIDATE} writes, and states no machine name "
                            "for that generation, so the machine the child's command waits "
                            "for cannot be named")
                    endif()
                    _sce_codegen_plan_machine(_WRITER_MACHINE _UNUSED_DEPFILE
                        "${_WRITER_MACHINE_NAME_${_CANDIDATE}}" "${_LANGUAGE}")
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
            set(_WRITER_MACHINE_NAME_${_CHILD} "${_GENERATION_${_CHILD}_MACHINE_NAME}")
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
            # An in-line child's document is one the generator writes and names
            # after the machine it gives the child,
            # `<parent machine>__sce_synth_invoke__<id>`, an identifier as
            # written: the name of the file the plan names is that machine's.
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

    # The machine the generator named, read from the plan: the stem written as
    # an identifier, never the stem as STEM gives it.
    _sce_codegen_plan_machine(_MACHINE _DEPFILE_NAME "${_MACHINE_NAME}" "${_LANGUAGE}")
    if(NOT _SCP_FROM)
        # A document of a forge kind (an algorithm, a schema, an enum, a
        # bounded collection ...) is generated into files of its own name and
        # no state machine, and its plan states no machine name. What its
        # command produces is then the run's primary artefact: the first file
        # the answer names, since the answer lists the files in the order the
        # run wrote them and the generator writes its primary first and names
        # it the depfile's one target. A run whose plan states the machine it
        # writes is a statechart's, and is held below to the files of the
        # machine so named. A set's run and a harness's produce their primary
        # (which the generator likewise writes first), and so does a run of a
        # language whose machine this module does not name.
        if(NOT _ONE_DOCUMENT OR "${_MACHINE}" STREQUAL "")
            set(_MACHINE "")
            foreach(_NAME ${_FILES})
                if(NOT "${_NAME}" MATCHES "\\.scxml$")
                    set(_MACHINE "${_NAME}")
                    break()
                endif()
            endforeach()
            if("${_MACHINE}" STREQUAL "")
                message(FATAL_ERROR
                    "sce-codegen --plan ${_RUN} (${_LANGUAGE}) names no file the run "
                    "writes, so its command would produce nothing")
            endif()
            set(_DEPFILE_NAME "${_MACHINE}.d")
        endif()
    endif()
    set(_DEPFILE "${_OUTPUT_DIR}/${_DEPFILE_NAME}")
    foreach(_NAMED ${_MACHINE})
        if(NOT "${_NAMED}" IN_LIST _FILES)
            message(FATAL_ERROR
                "sce-codegen --plan generate for ${_STEM} (${_LANGUAGE}) does not name "
                "${_NAMED}, a file of the machine ${_MACHINE_NAME} it states and its "
                "generate command is declared to produce")
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

    # What the machines of this run need linked, kept against every file the
    # run writes: a target that compiles one of them links what its generation
    # requires, whichever route declared the generation and however the target
    # names the file (a path, or the generator expression a deferred route
    # hands its caller). A file has one producer, so one answer.
    foreach(_FILE ${_HEADERS} ${_SOURCES})
        set_property(GLOBAL PROPERTY "SCE_GENERATED_REQUIRES:${_FILE}" "${_REQUIRES}")
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
#   document, its files, its arguments, what its machines require, the name
#   of the machine it writes and the machine of the generation that writes
#   its document.
macro(_sce_codegen_plan_publish TO FROM)
    foreach(_SCPP_NAME FILES SYNTH CHILDREN STAGED GENERATIONS OUTPUTS BYPRODUCTS
            BYPRODUCTS_ARG DEPFILE HEADERS SOURCES REQUIRES DECLARED_ELSEWHERE DOCUMENT
            RUN FORMATS DEPENDS ARGS STEM MACHINE_NAME OUTPUT_DIR LANGUAGE CLAIM INPUT_ROOT)
        set(${TO}_${_SCPP_NAME} "${${FROM}${_SCPP_NAME}}" PARENT_SCOPE)
    endforeach()
    foreach(_SCPP_CHILD ${${FROM}GENERATIONS})
        foreach(_SCPP_NAME DOCUMENT FILES ARGS REQUIRES MACHINE_NAME WRITER)
            set(${TO}_GENERATION_${_SCPP_CHILD}_${_SCPP_NAME}
                "${${FROM}GENERATION_${_SCPP_CHILD}_${_SCPP_NAME}}" PARENT_SCOPE)
        endforeach()
    endforeach()
endmacro()

# _sce_codegen_generation_plan_options(<VAR> <PARSED> <CALLER>)
#
#   The options `sce_codegen_plan` is asked with for one generation whose
#   options `cmake_parse_arguments` parsed into `<PARSED>_*` in the caller's
#   scope: the one list `sce_codegen_generation` asks ahead with and keeps for
#   the declaration to plan with. A document's stem is its name when not
#   given, and the deploy is passed as the run's `--deploy`. CALLER names the
#   call in a refusal.
function(_sce_codegen_generation_plan_options VAR PARSED CALLER)
    set(_OPTIONS "")
    if(NOT "${${PARSED}_DOCUMENT}" STREQUAL "")
        set(_STEM "${${PARSED}_STEM}")
        if("${_STEM}" STREQUAL "")
            get_filename_component(_STEM "${${PARSED}_DOCUMENT}" NAME_WLE)
        endif()
        list(APPEND _OPTIONS STEM "${_STEM}" DOCUMENT "${${PARSED}_DOCUMENT}")
    elseif(NOT "${${PARSED}_STEM}" STREQUAL "")
        list(APPEND _OPTIONS STEM "${${PARSED}_STEM}")
    endif()
    if(NOT "${${PARSED}_DOCUMENTS}" STREQUAL "")
        list(APPEND _OPTIONS DOCUMENTS ${${PARSED}_DOCUMENTS})
    endif()
    if(NOT "${${PARSED}_MANIFEST}" STREQUAL "")
        list(APPEND _OPTIONS MANIFEST "${${PARSED}_MANIFEST}")
    endif()
    list(APPEND _OPTIONS
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

# sce_codegen_generation(<PREFIX>
#     DOCUMENT <document> | DOCUMENTS <document>... | MANIFEST <catalog>
#     OUTPUT_DIR <dir> LANGUAGE <language>
#     [STEM <stem>]
#     [DEPLOY <deploy.yaml>]
#     [INPUT_ROOT <dir>]
#     [CHILDREN_HELD_IN <dir>]
#     [CHILD_GENERATIONS]
#     [CLAIM <global property>]
#     [ARGS <arg>...]
#     [COMMENT <text>])
#
#   One generation into OUTPUT_DIR, declared from its plan: of DOCUMENT
#   (`generate`), of the document set DOCUMENTS compiled together
#   (`orchestrate`), or of the conformance harness of the fixture catalog
#   MANIFEST (`generate-conformance`). The command that runs it is declared
#   with exactly the files the plan names as its OUTPUT and BYPRODUCTS, the
#   depfile it asks the generator for as its DEPFILE, and what it reads before
#   that depfile exists (the documents or the catalog, the deploy, the staged
#   children, the generator) as its DEPENDS. Every file of the run has this
#   command as its one producer, and no file name is written at the call: a
#   run with a deploy declares the transport it writes beside the machine
#   because the plan names it.
#
#   The call asks the plan ahead (`sce_codegen_plan_request`) and declares
#   nothing. Every generation called in a directory is declared when the
#   directory ends (`_sce_codegen_generations_declare`), after every plan
#   queued by then was sent to the generator in one batch: a plan is never
#   asked alone, and a caller has nothing to ask ahead itself. A call after
#   the generations of its directory were declared is refused, as one the
#   build would never run.
#
#   STEM     The document's name without `.scxml`, as its file writes it,
#            when not given: the generation's name in what the module says.
#            The machine's name, which names its files, is the generator's,
#            read from the plan (`sce_codegen_plan`, `<PREFIX>_MACHINE_NAME`).
#            One document's alone.
#   DEPLOY   The deploy the run is generated with (`--deploy`), which the
#            plan is asked with too and the command depends on.
#   CHILD_GENERATIONS
#            Plan, and declare, the generation of every child document the
#            run writes or stages and of every child document those write in
#            turn (`sce_codegen_plan` says which), each a command of its own
#            run with the arguments and input root the plan states for it,
#            depending on its document and on the machine of the generation
#            that writes it.
#   ARGS     The rest of the arguments that change what the run emits
#            (`--partition <name>` ...). Formatting is the build's
#            (`sce_codegen_format_args`) and is given to the command alone.
#   COMMENT  What the build prints when it runs the command (a child
#            generation's says which child it generates).
#   The others are `sce_codegen_plan`'s.
#
#   Sets, in the caller's scope, as generator expressions the declaration
#   fills before the build system is generated, so a target's sources take
#   them as they are and nothing reads them while the tree is configured:
#     <PREFIX>_HEADERS   every header the run writes, and with
#                        CHILD_GENERATIONS every header its child generations
#                        write: what a target compiles of it
#     <PREFIX>_SOURCES   likewise every C translation unit
#     <PREFIX>_FILES     likewise every file, whatever it is: a C++
#                        harness's translation unit, and what a generation
#                        of a language no target here compiles writes, which a
#                        target that has to wait for it lists as sources it
#                        does not compile
function(sce_codegen_generation PREFIX)
    cmake_parse_arguments(PARSE_ARGV 1 _SCGA "CHILD_GENERATIONS"
        "STEM;DOCUMENT;MANIFEST;OUTPUT_DIR;LANGUAGE;DEPLOY;INPUT_ROOT;CHILDREN_HELD_IN;CLAIM;COMMENT"
        "DOCUMENTS;ARGS")
    if(_SCGA_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_codegen_generation(${PREFIX}): unexpected argument(s): "
            "${_SCGA_UNPARSED_ARGUMENTS}. A child document's generation is declared by "
            "the generation whose plan holds it (CHILD_GENERATIONS), not by a call.")
    endif()
    get_property(_DECLARED DIRECTORY PROPERTY _SCE_CODEGEN_GENERATIONS_DECLARED)
    if(_DECLARED)
        message(FATAL_ERROR
            "sce_codegen_generation(${PREFIX}) comes after the generations of "
            "${CMAKE_CURRENT_SOURCE_DIR} were declared: it would never be generated")
    endif()

    # The one list the plan is asked ahead with and planned with when it is
    # declared, kept for the declaration.
    _sce_codegen_generation_plan_options(_PLAN_OPTIONS _SCGA "sce_codegen_generation(${PREFIX})")
    sce_codegen_plan_request(${_PLAN_OPTIONS})

    get_property(_ID GLOBAL PROPERTY _SCE_CODEGEN_GENERATION_COUNT)
    if("${_ID}" STREQUAL "")
        set(_ID 0)
    endif()
    math(EXPR _ID "${_ID} + 1")
    set_property(GLOBAL PROPERTY _SCE_CODEGEN_GENERATION_COUNT "${_ID}")
    if(NOT TARGET sce_codegen_generations)
        # Holds, per generation, the files the declaration found it writes,
        # for the generator expressions each caller is handed: a property of
        # a target is the one place a generator expression reads a value set
        # after the expression was written.
        add_library(sce_codegen_generations INTERFACE)
    endif()
    get_property(_REGISTERED DIRECTORY PROPERTY _SCE_CODEGEN_GENERATIONS)
    if("${_REGISTERED}" STREQUAL "")
        # The first generation of this directory: its declaration, and every
        # later one's, is made when the directory ends.
        cmake_language(DEFER CALL _sce_codegen_generations_declare)
        if(CMAKE_CURRENT_SOURCE_DIR STREQUAL CMAKE_SOURCE_DIR)
            _sce_codegen_plan_settle_last()
        endif()
    endif()
    set_property(DIRECTORY APPEND PROPERTY _SCE_CODEGEN_GENERATIONS "${_ID}")
    set_property(GLOBAL PROPERTY "_SCE_CODEGEN_GENERATION_${_ID}_PLAN_OPTIONS" "${_PLAN_OPTIONS}")
    foreach(_KEPT DEPLOY INPUT_ROOT COMMENT)
        set_property(GLOBAL PROPERTY "_SCE_CODEGEN_GENERATION_${_ID}_${_KEPT}" "${_SCGA_${_KEPT}}")
    endforeach()
    foreach(_KIND HEADERS SOURCES FILES)
        set(${PREFIX}_${_KIND}
            "$<TARGET_PROPERTY:sce_codegen_generations,SCE_GENERATION_${_ID}_${_KIND}>"
            PARENT_SCOPE)
    endforeach()
endfunction()

# _sce_codegen_generations_declare()
#
#   At the end of a directory: send every plan queued and not kept to the
#   generator in one batch, then declare every generation
#   `sce_codegen_generation` was called for in the directory, with every
#   child generation a CHILD_GENERATIONS plan holds, and fill the headers,
#   sources and files each call's generator expressions name.
function(_sce_codegen_generations_declare)
    set_property(DIRECTORY PROPERTY _SCE_CODEGEN_GENERATIONS_DECLARED TRUE)
    sce_codegen_plan_prefetch()
    get_property(_IDS DIRECTORY PROPERTY _SCE_CODEGEN_GENERATIONS)
    foreach(_ID IN LISTS _IDS)
        _sce_codegen_generation_declare(_RUN ${_ID})
        set(_HEADERS ${_RUN_HEADERS})
        set(_SOURCES ${_RUN_SOURCES})
        _sce_codegen_generation_files(_FILES _RUN)
        foreach(_CHILD IN LISTS _RUN_GENERATIONS)
            _sce_codegen_generation_declare(_CHILD_RUN ${_ID} FROM _RUN CHILD "${_CHILD}")
            list(APPEND _HEADERS ${_CHILD_RUN_HEADERS})
            list(APPEND _SOURCES ${_CHILD_RUN_SOURCES})
            _sce_codegen_generation_files(_CHILD_FILES _CHILD_RUN)
            list(APPEND _FILES ${_CHILD_FILES})
        endforeach()
        list(REMOVE_DUPLICATES _FILES)
        set_property(TARGET sce_codegen_generations
            PROPERTY "SCE_GENERATION_${_ID}_HEADERS" ${_HEADERS})
        set_property(TARGET sce_codegen_generations
            PROPERTY "SCE_GENERATION_${_ID}_SOURCES" ${_SOURCES})
        set_property(TARGET sce_codegen_generations
            PROPERTY "SCE_GENERATION_${_ID}_FILES" ${_FILES})
    endforeach()
endfunction()

# _sce_codegen_generation_files(<VAR> <PLAN>)
#
#   The path of every file the generation the plan <PLAN> declares writes:
#   each of `<PLAN>_FILES` in its output directory.
function(_sce_codegen_generation_files VAR PLAN)
    set(_PATHS "")
    foreach(_NAME IN LISTS ${PLAN}_FILES)
        list(APPEND _PATHS "${${PLAN}_OUTPUT_DIR}/${_NAME}")
    endforeach()
    set(${VAR} "${_PATHS}" PARENT_SCOPE)
endfunction()

# _sce_codegen_generation_declare(<PREFIX> <ID> [FROM <plan> CHILD <name>])
#
#   Declare the command of the generation `sce_codegen_generation` kept as
#   <ID>, from the plan asked with the options it was asked ahead with; or,
#   with FROM and CHILD, of the generation of the child document <name> the
#   plan <plan> of that generation holds, run with the arguments and input
#   root the plan states for it. Sets, in the caller's scope, every variable
#   `sce_codegen_plan` sets, as <PREFIX>_<NAME>. Called by
#   `_sce_codegen_generations_declare` alone, after the batch.
function(_sce_codegen_generation_declare PREFIX ID)
    cmake_parse_arguments(PARSE_ARGV 2 _SCGD "" "FROM;CHILD" "")
    set(_KEPT "_SCE_CODEGEN_GENERATION_${ID}")
    set(_DEPENDS "")
    set(_INPUT_ROOT_ARGS "")
    if(_SCGD_FROM)
        sce_codegen_plan(_SCGA_PLAN FROM "${_SCGD_FROM}" CHILD "${_SCGD_CHILD}")
        # A child's document is read from the output directory, and the
        # source set it was planned with is the run's.
        if(_SCGA_PLAN_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_SCGA_PLAN_INPUT_ROOT}")
        endif()
        string(CONCAT _COMMENT
            "Generating ${_SCGA_PLAN_STEM} (${_SCGA_PLAN_LANGUAGE}), a child generation "
            "the plan of ${${_SCGD_FROM}_STEM} holds")
    else()
        get_property(_PLAN_OPTIONS GLOBAL PROPERTY "${_KEPT}_PLAN_OPTIONS")
        get_property(_DEPLOY GLOBAL PROPERTY "${_KEPT}_DEPLOY")
        get_property(_INPUT_ROOT GLOBAL PROPERTY "${_KEPT}_INPUT_ROOT")
        get_property(_COMMENT GLOBAL PROPERTY "${_KEPT}_COMMENT")
        if(_DEPLOY)
            list(APPEND _DEPENDS "${_DEPLOY}")
        endif()
        sce_codegen_plan(_SCGA_PLAN ${_PLAN_OPTIONS})
        if(_INPUT_ROOT)
            set(_INPUT_ROOT_ARGS --input-root "${_INPUT_ROOT}")
        endif()
        if("${_COMMENT}" STREQUAL "")
            # A document's machine by its name; a set or a harness by the
            # first file it writes.
            set(_NAMED "${_SCGA_PLAN_STEM}")
            if("${_NAMED}" STREQUAL "")
                list(GET _SCGA_PLAN_OUTPUTS 0 _PRIMARY)
                get_filename_component(_NAMED "${_PRIMARY}" NAME)
            endif()
            set(_COMMENT "Generating ${_NAMED} (${_SCGA_PLAN_LANGUAGE})")
        endif()
    endif()
    list(APPEND _DEPENDS ${_SCGA_PLAN_DEPENDS})

    # A machine another plan of the same claim declared is that plan's
    # command's to write.
    if(NOT _SCGA_PLAN_DECLARED_ELSEWHERE)
        # The build's formatting, for a command that formats what it writes
        # (a harness is written as its templates write it), and the
        # environment the module gives every run of the generator.
        set(_FORMAT_ARGS "")
        if(_SCGA_PLAN_FORMATS)
            sce_codegen_format_args(_FORMAT_ARGS)
        endif()
        _sce_codegen_launcher(_LAUNCH)
        add_custom_command(
            OUTPUT ${_SCGA_PLAN_OUTPUTS}
            COMMAND ${_LAUNCH} "${SCE_CODEGEN}" ${_SCGA_PLAN_RUN}
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
