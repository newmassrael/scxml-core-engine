# SCECodegen.cmake
# Provides sce_add_state_machine() function for automatic SCXML code generation
#
# This file works both:
#   1. In-tree: When SCE is included as subdirectory (add_subdirectory)
#   2. Installed: When SCE is found via find_package(SCE)
#
# Uses sce-codegen (Rust binary) for code generation.
# Build: cargo build --bin sce-codegen --features cli -p sce-build
#
# Usage:
#   sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
#   sce_add_state_machines_from_dir(TARGET my_app SCXML_DIR scxml/)
#   target_link_libraries(my_app PRIVATE SCE::sce_base)
#
# What the generated C++ compiles against: the target links one SCE tier,
# SCE::sce_base for a machine that needs no script engine (`datamodel="sce-static"`
# or no expressions), SCE::sce_scripting for one that does, SCE::sce_runtime for
# the interpreter. Each tier carries everything the generated header includes:
# SCE's headers through SCE::sce_core, and the Forge C++ runtime
# (<sce/forge/algorithm.h>, <sce/forge/optional.h>, <sce/forge/span.h>) through
# SCE::sce_forge_runtime, which SCE::sce_core links. Nothing else is linked or
# added to the include path. The interface is C++17 (docs/adr/0059), so the
# consumer's own standard may be C++17; a tier never raises it.
# tests/install_tree_consumer builds a generated machine from an installed SCE
# this way.
#
# Every function below declares its generation through
# `sce_codegen_generation` (SCECodegenPlan.cmake, installed beside this file),
# the one declaration the SCE tree's own builds use: the files a generation
# writes are the generator's answer when the build is configured
# (`sce-codegen --plan generate`), not names written here, so a document that
# writes more than its machine (a child an inline `<invoke>` synthesizes, the
# header of a declared transition function, a hybrid `<invoke>`'s stub and the
# child generated from it) builds as it does in the tree. Each file has one
# command as its producer, which asks the generator for a depfile, depends on
# the document and the generator, and is declared when the directory it is
# called in ends, after the plans of every generation of that directory were
# asked in one batch. A target is handed the files as generator expressions,
# which is why the functions add them to the target rather than return their
# names. What the machines need linked is recorded against each file
# (`SCE_GENERATED_REQUIRES:<path>`, `<PREFIX>_REQUIRES` in SCECodegenPlan.cmake).

# The declaration gives each command a DEPFILE. A consumer's own
# cmake_minimum_required can be lower than this repository's, so the floor is
# enforced here, where a consumer meets it, and not left to surface as
# "Option DEPFILE not supported by Unix Makefiles" from inside a function (see
# the top CMakeLists.txt for the measurements behind 3.20). The policy the
# depfile reaches Ninja under (CMP0116) is set where the declaration is
# defined, in SCECodegenPlan.cmake.
if(CMAKE_VERSION VERSION_LESS 3.20)
    message(FATAL_ERROR
        "SCE: sce_add_state_machine() needs CMake 3.20 or newer for "
        "add_custom_command(DEPFILE); this is CMake ${CMAKE_VERSION}")
endif()

# Find sce-codegen binary. The search itself lives in SCEFindCodegen so
# the forge conformance and round-trip harnesses — which need the
# binary but none of the generation functions below — resolve it the
# same way rather than each naming a profile.
include("${CMAKE_CURRENT_LIST_DIR}/SCEFindCodegen.cmake")

message(STATUS "SCE: Using code generator: ${SCE_CODEGEN}")

# Auto-detect SCE_TEMPLATE_DIR so installed sce-codegen binaries (which
# refuse silent fallback — see sce-build::find_template_base) find the
# Jinja2 templates in all three distribution layouts without the
# consumer having to set SCE_TEMPLATE_DIR manually:
#
#   In-tree:        cmake/      <-> ../tools/codegen/templates
#   Embed vendor:   embed/      <-> tools/codegen/templates (alongside)
#   System install: lib/cmake/SCE/ <-> ../../share/sce/codegen/templates
#
# Consumer-set SCE_TEMPLATE_DIR wins; auto-detection only runs when unset.
# Detected before SCECodegenPlan.cmake is included below, which gives it to
# every run of the generator, a plan and a generation alike.
if(NOT SCE_TEMPLATE_DIR)
    if(EXISTS "${CMAKE_CURRENT_LIST_DIR}/../tools/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/../tools/codegen/templates")
    elseif(EXISTS "${CMAKE_CURRENT_LIST_DIR}/tools/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/tools/codegen/templates")
    elseif(EXISTS "${CMAKE_CURRENT_LIST_DIR}/../../share/sce/codegen/templates/state_machine.jinja2")
        set(SCE_TEMPLATE_DIR "${CMAKE_CURRENT_LIST_DIR}/../../share/sce/codegen/templates")
    endif()
    if(SCE_TEMPLATE_DIR)
        message(STATUS "SCE: Auto-detected template dir: ${SCE_TEMPLATE_DIR}")
    endif()
endif()

# The one declaration of a generation (and, through it, the build's
# formatting of generated C++, SCEClangFormat.cmake).
include("${CMAKE_CURRENT_LIST_DIR}/SCECodegenPlan.cmake")

# _sce_codegen_document(<VAR> <file> <caller>)
#
#   The document a public function is handed, as the generator reads it: the
#   library name of a standard statechart (sce:std/shared/record_list.scxml)
#   as written, since the generator resolves it from the library it embeds
#   and the build depends on the generator; any other path made absolute and
#   refused when there is no such file.
function(_sce_codegen_document VAR FILE CALLER)
    if("${FILE}" MATCHES "^sce:")
        set(${VAR} "${FILE}" PARENT_SCOPE)
        return()
    endif()
    get_filename_component(_ABSOLUTE "${FILE}" ABSOLUTE)
    if(NOT EXISTS "${_ABSOLUTE}")
        message(FATAL_ERROR "${CALLER}: SCXML file not found: ${_ABSOLUTE}")
    endif()
    set(${VAR} "${_ABSOLUTE}" PARENT_SCOPE)
endfunction()

# _sce_codegen_consumer_claim(<VAR> <output dir>)
#
#   The list the generations of a public function claim in: one per output
#   directory, so a file two of them would write into it (a child two machines
#   start, a machine added to two targets) has the first's command as its one
#   producer, and the second declares none of its own.
function(_sce_codegen_consumer_claim VAR OUTPUT_DIR)
    get_filename_component(_DIR "${OUTPUT_DIR}" ABSOLUTE)
    set(${VAR} "SCE_CODEGEN_DECLARED_OUTPUTS:${_DIR}" PARENT_SCOPE)
endfunction()

# _sce_codegen_add_generation(<target> <PREFIX> <language> <output dir> <name>)
#
#   Give <target> what the generation `sce_codegen_generation(<PREFIX> ...)`
#   declared: a C++ or C11 target compiles its headers and C sources
#   (`<PREFIX>_HEADERS`, `<PREFIX>_SOURCES`) and finds them on its include
#   path. The code of another language (Kotlin, Rust, Go, Python) is compiled
#   by a build of its own, so the target only waits for it: a custom target
#   named `<target>_<name>_kt` for Kotlin, `<target>_<name>_<language>`
#   otherwise, lists every file the generation writes (`<PREFIX>_FILES`).
function(_sce_codegen_add_generation TARGET PREFIX LANGUAGE OUTPUT_DIR NAME)
    if(LANGUAGE STREQUAL "cpp" OR LANGUAGE STREQUAL "c11" OR LANGUAGE STREQUAL "c")
        target_sources(${TARGET} PRIVATE ${${PREFIX}_HEADERS} ${${PREFIX}_SOURCES})
        target_include_directories(${TARGET} PRIVATE "${OUTPUT_DIR}")
        return()
    endif()
    set(_SUFFIX "${LANGUAGE}")
    if(LANGUAGE STREQUAL "kotlin")
        set(_SUFFIX kt)
    endif()
    set(_WAITED_FOR "${TARGET}_${NAME}_${_SUFFIX}")
    add_custom_target(${_WAITED_FOR})
    target_sources(${_WAITED_FOR} PRIVATE ${${PREFIX}_FILES})
    add_dependencies(${TARGET} ${_WAITED_FOR})
endfunction()

#[=============================================================================[
sce_add_state_machine(TARGET target SCXML_FILE file.scxml [OUTPUT_DIR dir] [LANGUAGE lang] [CPP_NAMESPACE_PREFIX prefix] [AS_CHILD])

Generates state machine code from SCXML and adds to target.

Arguments:
  TARGET      - CMake target to add generated code to (required)
  SCXML_FILE  - Path to SCXML file, or the library name of a standard statechart
                such as sce:std/shared/record_list.scxml (required)
  AS_CHILD   - Generate the machine as a child another machine invokes
                (`sce-codegen generate --as-child`): a class template over the
                parent's type, which is the form a parent that starts it names.
                A document a hybrid `<invoke>` declares in `sce:candidates` is
                one — the parent refers to it as
                `::SCE::Generated::<machine>::<machine><SelfType>` — so it is
                generated this way and not as a machine of its own (optional,
                cpp only). The parent's own generation already declares the
                generation of every child it names (see below), so a target
                that compiles the parent needs no call for its children.
  OUTPUT_DIR  - Output directory for generated files (optional, defaults to
                ${CMAKE_CURRENT_BINARY_DIR}/generated)
  LANGUAGE    - Target language: cpp (default), c11, rust, kotlin, go or python
  SCRIPT_ENGINE_LANGUAGE - Which language the generated machine hands its script
                engine: `lua` (the build-time frontend lowers every expression)
                or `ecmascript` (the author's text crosses to the engine, which
                adapts or refuses). Omit it and the backend emits for the engine
                it already emitted for, so the command stays byte-identical to
                one from before this argument existed. A backend that cannot
                honour the request refuses rather than emit an artifact half in
                each language — see docs/SCE_LUA_TRANSLATION_SEAM.md. This must
                agree with the SCE_SCRIPT_ENGINE the runtime was configured
                with: a Lua-shaped artifact can only run on a Lua engine.
                Spelled with `_LANGUAGE` on purpose — see the collision note in
                the body — and it is the manifest's own field name.
  CPP_NAMESPACE_PREFIX - Nest the emitted C++ namespace under
                SCE::Generated::<prefix>::<machine> instead of the default
                SCE::Generated::<machine>, so a separate catalog can reuse
                in-tree machine names without an ODR clash (optional, cpp only;
                ignored for other languages and when unset)
  HOST_PROCESSOR / HOST_INVOKER - The Event I/O Processor types and the
                `<invoke>` types the build's host serves (optional,
                multi-valued; `--host-processor`, `--host-invoker`)

The machine's name is the generator's: the file's stem written as an
identifier (a hyphen or a dot becomes an underscore, a capital after a
lower-case letter or a digit opens a word, every letter is lower case), which
names its files, its namespace and its types, and which the plan states when
the build is configured. `job-tester.scxml` is the machine `job_tester`: the
target compiles `job_tester_sm.h` (and, for c11, `job_tester_sm.c`), whose
machine is in `namespace SCE::Generated::job_tester`. A stem already in lower
snake case is the machine's name as written.

What the target is given: for cpp and c11, every header (and C11 source) the
generation writes, the machine's and those of every child generation its plan
holds (a child an inline `<invoke>` synthesizes, a `src` or candidate child
the machine starts as a class template over its type), and the output
directory on its include path. For another language, whose code a build of
its own compiles, the target waits for the generation: it depends on a custom
target `<target>_<name>_kt` (Kotlin) or `<target>_<name>_<language>` that lists
every file the generation writes, where `<name>` is the file's name up to its
first dot, as written (`<target>_job-tester_kt`): a target is named when the
call is made, before the plan that states the machine's name is read, and a
CMake target's name takes a hyphen. Machines added into one output directory
share it: a file two of them would write is generated once.

Example:
  add_executable(my_app main.cpp)
  sce_add_state_machine(TARGET my_app SCXML_FILE player.scxml)
  # Link the tier matching the SCXML feature set (ARCHITECTURE.md §4-Tier):
  #   SCE::sce_base      — pure static (no <cond>/<expr>/<assign location>,
  #                        or datamodel="sce-static")
  #   SCE::sce_scripting — static hybrid (needs_script_engine=true)
  #   SCE::sce_runtime   — full interpreter (dynamic SCXML loading)
  # The tier is all the generated header needs: it carries the Forge C++
  # runtime (SCE::sce_forge_runtime) the header includes.
  target_link_libraries(my_app PRIVATE SCE::sce_base)
#]=============================================================================]
function(sce_add_state_machine)
    cmake_parse_arguments(SCE "AS_CHILD" "TARGET;SCXML_FILE;OUTPUT_DIR;LANGUAGE;SCRIPT_ENGINE_LANGUAGE;CPP_NAMESPACE_PREFIX" "HOST_PROCESSOR;HOST_INVOKER" ${ARGN})

    # ⚠ The parse prefix is `SCE`, so a keyword may not name an SCE CACHE entry.
    #
    # The engine-language argument was first spelled `SCRIPT_ENGINE`, which
    # parses into `SCE_SCRIPT_ENGINE` — this tree's own engine-selection cache
    # option. `cmake_parse_arguments` UNSETS the variable for a keyword the
    # caller omitted, and unsetting a normal variable reveals the cache entry
    # underneath, so an omitted argument silently became "whatever engine this
    # tree was configured with". Measured 2026-08-29 while building the red
    # witness for `scripts/gate ecma262-lowered-cpp`: dropping the argument from
    # the lowered target left its artifact fully lowered anyway — 170
    # `ScriptSource::lua(...)` pairs — because the tree was configured
    # `-DSCE_SCRIPT_ENGINE=lua`. A gate cannot be shown to catch a lost flag
    # while the flag cannot be lost.
    #
    # Asserted rather than only written down, because the next keyword added
    # here is one rename away from the same silence.
    if(DEFINED CACHE{SCE_SCRIPT_ENGINE_LANGUAGE})
        message(FATAL_ERROR
            "sce_add_state_machine: SCE_SCRIPT_ENGINE_LANGUAGE exists as a cache "
            "entry, so an omitted SCRIPT_ENGINE_LANGUAGE argument would inherit "
            "it instead of leaving the backend's default. Rename the cache entry "
            "or the keyword.")
    endif()

    if(SCE_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_add_state_machine: unexpected argument(s): ${SCE_UNPARSED_ARGUMENTS}")
    endif()

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machine: TARGET is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_add_state_machine: SCXML_FILE is required")
    endif()

    # Verify target exists
    if(NOT TARGET ${SCE_TARGET})
        message(FATAL_ERROR "sce_add_state_machine: TARGET '${SCE_TARGET}' does not exist")
    endif()

    # Set default language
    if(NOT SCE_LANGUAGE)
        set(SCE_LANGUAGE "cpp")
    endif()

    # An artifact in this tree can only run on the engine this tree compiled
    # in, so when the caller does not name a language for the artifact to hand
    # its engine, take the one that engine SPEAKS.
    #
    # `SCE_SCRIPT_ENGINE` is a cache option on the C++ runtime library and the
    # definition it sets is PUBLIC on `sce_scripting`, so the selection is a
    # property of the whole C++ tree — and of nothing else. That is why this is
    # scoped to `cpp`: a Rust, Go, Python or Kotlin artifact generated here runs
    # against whatever engine ITS host supplies, and this cache entry says
    # nothing about that host. Deriving for them would also hand
    # `--script-engine lua` to a backend that may refuse it
    # (`Language::supports_script_engine_target`), turning an unrelated cache
    # value into a build failure.
    #
    # `quickjs` is deliberately left UNSET rather than spelled `ecmascript`:
    # omitting the flag is what keeps a run byte-identical to one from before
    # the flag existed, which is the rule `sce-codegen`'s own flag follows.
    #
    # ⚠ This changes what a `-DSCE_SCRIPT_ENGINE=lua` tree emits, and that is
    # the point — such a tree used to hand its Lua engine the author's
    # ECMAScript and take the runtime rewriter's answers, which
    # `tests/ecmascript/lua_engine_divergences.json` enumerates case by case.
    # It does NOT by itself empty that file. Measured 2026-08-29: the two suites
    # holding that list reach the engine by routes this default cannot touch —
    # `ecmascript_semantics_test` calls `LuaEngine::instance()` directly and
    # carries no generated machine at all, and `LoweredEcma262`'s control names
    # `SCRIPT_ENGINE_LANGUAGE ecmascript` explicitly so that it stays a control.
    # The list empties when the rewriter is RETIRED; this is the step that
    # removes generated C++ as its consumer.
    if(NOT SCE_SCRIPT_ENGINE_LANGUAGE AND SCE_LANGUAGE STREQUAL "cpp")
        string(TOLOWER "${SCE_SCRIPT_ENGINE}" _SCE_TREE_ENGINE_ID)
        if(_SCE_TREE_ENGINE_ID STREQUAL "lua")
            set(SCE_SCRIPT_ENGINE_LANGUAGE "lua")
        endif()
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # The document's name as its file writes it, for what the build says and
    # for the name of the target a generation of another language is waited
    # on through. The machine's name and its files are the generator's, which
    # the plan states (`<PREFIX>_MACHINE_NAME` in SCECodegenPlan.cmake).
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)
    _sce_codegen_document(_SCE_DOCUMENT "${SCE_SCXML_FILE}" sce_add_state_machine)

    # The arguments that change what the run emits, each given only when
    # asked, so a call that names none runs the generation it did before the
    # argument existed.
    set(_SCE_ARGS "")
    # Optional C++ namespace nesting: SCE::Generated::<prefix>::<machine>.
    # Lets a separate catalog reuse in-tree machine names without an ODR clash
    # when both link into one binary. cpp-only (mirrors sce-codegen's
    # --cpp-namespace-prefix, which is meaningful only with -l cpp).
    if(SCE_CPP_NAMESPACE_PREFIX AND SCE_LANGUAGE STREQUAL "cpp")
        list(APPEND _SCE_ARGS --cpp-namespace-prefix "${SCE_CPP_NAMESPACE_PREFIX}")
    endif()
    # A machine another one invokes, as the parent names it (a class template
    # over the parent's type).
    if(SCE_AS_CHILD)
        list(APPEND _SCE_ARGS --as-child)
    endif()
    # Which language the artifact hands its engine.
    if(SCE_SCRIPT_ENGINE_LANGUAGE)
        list(APPEND _SCE_ARGS --script-engine "${SCE_SCRIPT_ENGINE_LANGUAGE}")
    endif()
    # §scxml-6.2.5: the Event I/O Processor types this build's HOST serves.
    # Multi-valued because the identifier set is open and a host may serve
    # several. Codegen decides at compile time whether a `<send type>` site
    # dispatches or refuses, so a host that registers a handler without
    # declaring the type here still meets the refusal — which is the point:
    # the two halves have to agree, and only one of them is in the build.
    foreach(_sce_host_processor IN LISTS SCE_HOST_PROCESSOR)
        list(APPEND _SCE_ARGS --host-processor "${_sce_host_processor}")
    endforeach()
    # §scxml-6.4.1: the invoke half, declared separately for the reason the
    # flags are separate — delivering an event is not the same capability as
    # running a process with a lifetime, and one list would make declaring
    # either silently claim both.
    foreach(_sce_host_invoker IN LISTS SCE_HOST_INVOKER)
        list(APPEND _SCE_ARGS --host-invoker "${_sce_host_invoker}")
    endforeach()

    # The children a C++ or C11 machine starts are planned and declared with
    # it: a target that compiles the machine compiles the headers the machine
    # includes. The module names the machine of no other language, and their
    # builds compile what the generation writes on their own.
    set(_SCE_CHILDREN "")
    if(SCE_LANGUAGE STREQUAL "cpp" OR SCE_LANGUAGE STREQUAL "c11")
        set(_SCE_CHILDREN CHILD_GENERATIONS)
    endif()

    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")
    _sce_codegen_consumer_claim(_SCE_CLAIM "${SCE_OUTPUT_DIR}")
    sce_codegen_generation(_SCE_MACHINE
        DOCUMENT "${_SCE_DOCUMENT}"
        OUTPUT_DIR "${SCE_OUTPUT_DIR}"
        LANGUAGE "${SCE_LANGUAGE}"
        ${_SCE_CHILDREN}
        CLAIM "${_SCE_CLAIM}"
        COMMENT "SCE: Generating ${SCXML_NAME} (${SCE_LANGUAGE}) from SCXML"
        ARGS ${_SCE_ARGS})
    _sce_codegen_add_generation(${SCE_TARGET} _SCE_MACHINE "${SCE_LANGUAGE}"
        "${SCE_OUTPUT_DIR}" "${SCXML_NAME}")

    message(STATUS "SCE: Added state machine '${SCXML_NAME}' (${SCE_LANGUAGE}) to target '${SCE_TARGET}'")
endfunction()

#[=============================================================================[
sce_add_algorithm(TARGET target SCXML_FILE file.scxml [OUTPUT_DIR dir])

Generates the C++ header of a forge algorithm and adds it to target.

A statechart that calls an imported algorithm (datamodel="sce-static")
includes the header its own generation names, and does not generate it: the
algorithm is a document of its own kind, generated once however many machines
call it. This puts that header where the machine's is.

Arguments:
  TARGET      - CMake target the header is added to (required)
  SCXML_FILE  - Path to the algorithm's SCXML file, or a library name such as
                sce:std/sync/sync_failure.scxml (required)
  OUTPUT_DIR  - Output directory (optional, defaults to the one
                sce_add_state_machine uses, so a machine finds its algorithms)

The header is named by the generator (the algorithm's `name` in snake_case
with `.h`, which the including machine spells in its `#include`), and its
plan says so: the build does not name it. HEADER, which used to name it here,
is refused, since a second spelling of the generator's name is one that falls
behind it.

Example:
  sce_add_state_machine(TARGET my_app SCXML_FILE thermostat.scxml)
  sce_add_algorithm(TARGET my_app SCXML_FILE clamp.scxml)
#]=============================================================================]
function(sce_add_algorithm)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_FILE;OUTPUT_DIR;HEADER" "" ${ARGN})

    if(SCE_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_add_algorithm: unexpected argument(s): ${SCE_UNPARSED_ARGUMENTS}")
    endif()
    if(DEFINED SCE_HEADER)
        message(FATAL_ERROR
            "sce_add_algorithm: HEADER ${SCE_HEADER} is the generator's to name, and its "
            "plan names it when the build is configured: drop HEADER from the call")
    endif()
    foreach(_sce_required TARGET SCXML_FILE)
        if(NOT SCE_${_sce_required})
            message(FATAL_ERROR "sce_add_algorithm: ${_sce_required} is required")
        endif()
    endforeach()
    if(NOT TARGET ${SCE_TARGET})
        message(FATAL_ERROR "sce_add_algorithm: TARGET '${SCE_TARGET}' does not exist")
    endif()
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    get_filename_component(_SCE_ALGORITHM_NAME "${SCE_SCXML_FILE}" NAME_WE)
    _sce_codegen_document(_SCE_ALGORITHM_DOCUMENT "${SCE_SCXML_FILE}" sce_add_algorithm)

    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")
    _sce_codegen_consumer_claim(_SCE_CLAIM "${SCE_OUTPUT_DIR}")
    sce_codegen_generation(_SCE_ALGORITHM
        DOCUMENT "${_SCE_ALGORITHM_DOCUMENT}"
        OUTPUT_DIR "${SCE_OUTPUT_DIR}"
        LANGUAGE cpp
        CLAIM "${_SCE_CLAIM}"
        COMMENT "SCE: Generating algorithm ${_SCE_ALGORITHM_NAME} from ${SCE_SCXML_FILE}")
    _sce_codegen_add_generation(${SCE_TARGET} _SCE_ALGORITHM cpp
        "${SCE_OUTPUT_DIR}" "${_SCE_ALGORITHM_NAME}")

    message(STATUS "SCE: Added algorithm '${_SCE_ALGORITHM_NAME}' to target '${SCE_TARGET}'")
endfunction()

#[=============================================================================[
sce_add_state_machines_from_dir(TARGET target SCXML_DIR dir [OUTPUT_DIR dir] [LANGUAGE lang] [CPP_NAMESPACE_PREFIX prefix])

Finds all *.scxml files in directory and generates state machines.

Arguments:
  TARGET      - CMake target to add generated code to (required)
  SCXML_DIR   - Directory containing SCXML files (required)
  OUTPUT_DIR  - Output directory for generated files (optional)
  LANGUAGE    - Target language: cpp (default), c11, rust, kotlin, go or python
                (forwarded to each per-file sce_add_state_machine call)
  SCRIPT_ENGINE_LANGUAGE - Forwarded to each per-file sce_add_state_machine call
                (optional; see sce_add_state_machine)
  CPP_NAMESPACE_PREFIX - Forwarded to each per-file sce_add_state_machine call
                (optional, cpp only; see sce_add_state_machine)

NOTE: This function uses file(GLOB) which only runs at configure time.
      If you add new SCXML files to the directory, you must reconfigure
      (run cmake again) for them to be detected. For explicit control,
      use sce_add_state_machine() for each file instead.

Example:
  add_executable(my_app main.cpp)
  sce_add_state_machines_from_dir(TARGET my_app SCXML_DIR ${CMAKE_SOURCE_DIR}/scxml)
#]=============================================================================]
function(sce_add_state_machines_from_dir)
    cmake_parse_arguments(SCE "" "TARGET;SCXML_DIR;OUTPUT_DIR;LANGUAGE;SCRIPT_ENGINE_LANGUAGE;CPP_NAMESPACE_PREFIX" "" ${ARGN})

    # Validate required arguments
    if(NOT SCE_TARGET)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: TARGET is required")
    endif()

    if(NOT SCE_SCXML_DIR)
        message(FATAL_ERROR "sce_add_state_machines_from_dir: SCXML_DIR is required")
    endif()

    # Set default language
    if(NOT SCE_LANGUAGE)
        set(SCE_LANGUAGE "cpp")
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # Find all SCXML files
    file(GLOB SCXML_FILES "${SCE_SCXML_DIR}/*.scxml")

    if(NOT SCXML_FILES)
        message(WARNING "SCE: No SCXML files found in ${SCE_SCXML_DIR}")
        return()
    endif()

    # Generate state machine for each SCXML file
    foreach(SCXML_FILE ${SCXML_FILES})
        sce_add_state_machine(
            TARGET ${SCE_TARGET}
            SCXML_FILE ${SCXML_FILE}
            OUTPUT_DIR ${SCE_OUTPUT_DIR}
            LANGUAGE ${SCE_LANGUAGE}
            SCRIPT_ENGINE_LANGUAGE "${SCE_SCRIPT_ENGINE_LANGUAGE}"
            CPP_NAMESPACE_PREFIX "${SCE_CPP_NAMESPACE_PREFIX}"
        )
    endforeach()

    list(LENGTH SCXML_FILES SCXML_COUNT)
    message(STATUS "SCE: Added ${SCXML_COUNT} state machines from ${SCE_SCXML_DIR}")
endfunction()

#[=============================================================================[
sce_create_state_machine_library(NAME name SCXML_FILE file.scxml [OUTPUT_DIR dir])

Creates a standalone INTERFACE library from SCXML file.
Useful when you want to share generated code between multiple targets.

Arguments:
  NAME        - Name for the generated library (required)
  SCXML_FILE  - Path to SCXML file, or the library name of a standard
                statechart (required)
  OUTPUT_DIR  - Output directory for generated files (optional)

The library carries the output directory on its interface include path and
depends on a custom target `<name>_gen` that lists every file the generation
writes, its child generations' included, so a target linking it waits for
them.

Example:
  sce_create_state_machine_library(NAME player_sm SCXML_FILE player.scxml)
  # Pick the SCE tier matching the SCXML feature set — see ARCHITECTURE.md §4-Tier.
  target_link_libraries(my_app PRIVATE player_sm SCE::sce_base)
#]=============================================================================]
function(sce_create_state_machine_library)
    cmake_parse_arguments(SCE "" "NAME;SCXML_FILE;OUTPUT_DIR" "" ${ARGN})

    if(SCE_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_create_state_machine_library: unexpected argument(s): "
            "${SCE_UNPARSED_ARGUMENTS}")
    endif()

    # Validate required arguments
    if(NOT SCE_NAME)
        message(FATAL_ERROR "sce_create_state_machine_library: NAME is required")
    endif()

    if(NOT SCE_SCXML_FILE)
        message(FATAL_ERROR "sce_create_state_machine_library: SCXML_FILE is required")
    endif()

    # Set default output directory
    if(NOT SCE_OUTPUT_DIR)
        set(SCE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
    endif()

    # The document's name as its file writes it, for what the build says; the
    # machine's files are the generator's, which the plan states.
    get_filename_component(SCXML_NAME "${SCE_SCXML_FILE}" NAME_WE)
    _sce_codegen_document(_SCE_DOCUMENT "${SCE_SCXML_FILE}" sce_create_state_machine_library)

    file(MAKE_DIRECTORY "${SCE_OUTPUT_DIR}")
    _sce_codegen_consumer_claim(_SCE_CLAIM "${SCE_OUTPUT_DIR}")
    sce_codegen_generation(_SCE_LIBRARY
        DOCUMENT "${_SCE_DOCUMENT}"
        OUTPUT_DIR "${SCE_OUTPUT_DIR}"
        LANGUAGE cpp
        CHILD_GENERATIONS
        CLAIM "${_SCE_CLAIM}"
        COMMENT "SCE: Generating the ${SCXML_NAME} library")

    # Every file the generation writes, for a target linking the library to
    # wait for.
    add_custom_target(${SCE_NAME}_gen)
    target_sources(${SCE_NAME}_gen PRIVATE ${_SCE_LIBRARY_FILES})

    add_library(${SCE_NAME} INTERFACE)
    add_dependencies(${SCE_NAME} ${SCE_NAME}_gen)
    target_include_directories(${SCE_NAME} INTERFACE "${SCE_OUTPUT_DIR}")

    message(STATUS "SCE: Created state machine library '${SCE_NAME}' from ${SCXML_NAME}.scxml")
endfunction()
