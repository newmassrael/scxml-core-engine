# SCETestFamilies.cmake
#
# Which families of tests a configure registers.
#
#   cmake -S . -B build -DSCE_TEST_FAMILIES="standard_host"
#
# SCE_TEST_FAMILIES is a list of the families below. Empty, the default, is
# every family, so a tree configured without it registers every test it ever
# did: the build machines, a developer's tree and every lane that runs the
# suites are configured that way. A family left out is not declared at all: no
# target, no test, no generation, and no plan of a generation asked of
# sce-codegen. A name that is not a family is refused, so a misspelt family
# cannot configure a tree that silently holds nothing.
#
# ── Why a configure is scoped, not its plans deferred ──────────────────
#
# A lane that builds one program configured the whole test tree for it, and
# the configure was most of its time: windows-msvc's ran 489 s (job
# 138963137937, 2026-11-25), 73 s of it building the lowering library and most
# of the rest declaring the 1221 generations of the tree, each planned by the
# generator, for the one program it builds; ecma262-lowered-cpp configured for
# 423 s a tree whose one target built in 81 s (job 139038611735). Each
# plan answers which files a generation writes (cmake/SCECodegenPlan.cmake
# says why that is the generator's answer and not a list kept here), and a
# build system has to know the files a target compiles before it generates
# the build: Ninja and MSBuild alike take a target's sources from the
# configure. Moving the plans into the build would make the sources of every
# target that compiles a generated machine unknown until it is built, which
# neither generator supports. What a lane can give up is the targets it does
# not build, and their plans go with them.
#
# ── The families ───────────────────────────────────────────────────────
#
# Every test the tree registers belongs to exactly one family, declared
# where its block begins with `if(SCE_TEST_FAMILY_<NAME>)`:
# `cmake_option_guard_scope` (sce-build/tests) holds every `add_test` of the
# test tree to lying under one family's guard, and holds every target, and
# every variable, a family's block declares to be named only under that
# family's guard, as it holds a build option's. The infrastructure every
# family shares stays outside every guard: GoogleTest, the test policies,
# and the staleness fixture `sce_build_is_current` at the end of
# tests/CMakeLists.txt, which requires every test that is registered.
#
# A family that needs another option (the Mesh transports, the Lua engine)
# still needs it: the family is a further condition on its block, never a
# replacement for the one it had.

include_guard(GLOBAL)

set(SCE_TEST_FAMILIES "" CACHE STRING
    "The test families to register (cmake/SCETestFamilies.cmake), as a list; empty registers every family")

set_property(GLOBAL PROPERTY _SCE_TEST_FAMILY_NAMES "")

# sce_test_family(<name> <what it registers>)
#
#   Declare the family <name>: set SCE_TEST_FAMILY_<NAME> in the scope of the
#   includer, which every directory the includer adds inherits, ON when
#   SCE_TEST_FAMILIES is empty or names <name> and OFF otherwise. <what it
#   registers> is said in the configure's summary of a scoped tree.
function(sce_test_family NAME WHAT)
    if(NOT NAME MATCHES "^[a-z][a-z0-9_]*$")
        message(FATAL_ERROR
            "sce_test_family: '${NAME}' is not a family name (lower case, digits and _)")
    endif()
    string(TOUPPER "${NAME}" _UPPER)
    if("${SCE_TEST_FAMILIES}" STREQUAL "" OR NAME IN_LIST SCE_TEST_FAMILIES)
        set(SCE_TEST_FAMILY_${_UPPER} ON PARENT_SCOPE)
    else()
        set(SCE_TEST_FAMILY_${_UPPER} OFF PARENT_SCOPE)
    endif()
    set_property(GLOBAL APPEND PROPERTY _SCE_TEST_FAMILY_NAMES "${NAME}")
    set_property(GLOBAL PROPERTY "_SCE_TEST_FAMILY_WHAT_${NAME}" "${WHAT}")
endfunction()

sce_test_family(engine
    "the engine's own suites: parser, interpreter, scripting, timers, the model, the TXML converter, parsing, template parity, benchmarks")
sce_test_family(static_datamodel
    "datamodel=sce-static under the interpreter and generated to C++, and the runtime suite runners")
sce_test_family(standard_host
    "the standard host features against the reference host, and the payload record codec")
sce_test_family(lowered_ecma262
    "ECMA-262 through a Lua-lowered C++ artifact, in a SCE_SCRIPT_ENGINE=lua tree")
sce_test_family(integration
    "integration_tests, the C++ integration fixtures and the C++ feature conformance programs")
sce_test_family(w3c
    "the W3C conformance programs on C++, the interactive suite and the runner's own reporting")
sce_test_family(mesh
    "the Mesh transports and their fixtures, with SCE_ENABLE_MESH")
sce_test_family(c11
    "the C11 backend's W3C conformance, integration fixtures and runtime suites (backends/c/tests)")

get_property(_SCE_TEST_FAMILY_NAMES GLOBAL PROPERTY _SCE_TEST_FAMILY_NAMES)
set(_SCE_TEST_FAMILIES_UNKNOWN "")
foreach(_SCE_TEST_FAMILY IN LISTS SCE_TEST_FAMILIES)
    if(NOT _SCE_TEST_FAMILY IN_LIST _SCE_TEST_FAMILY_NAMES)
        list(APPEND _SCE_TEST_FAMILIES_UNKNOWN "${_SCE_TEST_FAMILY}")
    endif()
endforeach()
if(_SCE_TEST_FAMILIES_UNKNOWN)
    string(JOIN ", " _SCE_TEST_FAMILIES_KNOWN ${_SCE_TEST_FAMILY_NAMES})
    message(FATAL_ERROR
        "SCE_TEST_FAMILIES names ${_SCE_TEST_FAMILIES_UNKNOWN}, which no family is; "
        "the families are ${_SCE_TEST_FAMILIES_KNOWN} (cmake/SCETestFamilies.cmake)")
endif()

# A scoped tree says what it registers and what it leaves out, so a log of
# the configure cannot be read as the whole suite's.
if(NOT "${SCE_TEST_FAMILIES}" STREQUAL "")
    foreach(_SCE_TEST_FAMILY IN LISTS _SCE_TEST_FAMILY_NAMES)
        get_property(_SCE_TEST_FAMILY_WHAT GLOBAL PROPERTY "_SCE_TEST_FAMILY_WHAT_${_SCE_TEST_FAMILY}")
        if(_SCE_TEST_FAMILY IN_LIST SCE_TEST_FAMILIES)
            message(STATUS "SCE: test family ${_SCE_TEST_FAMILY} registered: ${_SCE_TEST_FAMILY_WHAT}")
        else()
            message(STATUS "SCE: test family ${_SCE_TEST_FAMILY} left out (SCE_TEST_FAMILIES)")
        endif()
    endforeach()
endif()
unset(_SCE_TEST_FAMILY)
unset(_SCE_TEST_FAMILY_WHAT)
unset(_SCE_TEST_FAMILIES_UNKNOWN)
unset(_SCE_TEST_FAMILIES_KNOWN)
