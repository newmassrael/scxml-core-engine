# SCE W3C Registry
#
# Which W3C conformance fixtures this repository runs, and how each is run,
# read from the conformance registry (tests/w3c/conformance/fixtures.json),
# for every CMake route that registers them: the C++ one (tests/CMakeLists.txt)
# and the C11 one (backends/c/tests/CMakeLists.txt).
#
# The registry is the source of the set. A route that kept its own list of ids
# would be a second copy of it that can disagree with the first without
# anything failing: the C++ route was 202 hand-written registrations until the
# registry existed, and the C11 route was 202 more, in an order of their own
# and with the libraries each fixture links written beside each id, until
# 2026-11. `sce-build/tests/w3c_registry_drives_cmake.rs` holds both routes to
# reading the set from here.
#
# The ids are read through `sce-codegen list-fixtures`, which hands them over
# as a CMake list, the way every forge conformance harness consumes its own
# catalog, so there is no JSON parsing in CMake. This module runs no
# generation: which files a generation writes is asked of the generator in one
# place (cmake/SCECodegenPlan.cmake).
#
# Requires SCE_CODEGEN (cmake/SCEFindCodegen.cmake resolves it; the
# conformance route, cmake/SCEStaticW3CTest.cmake, includes this module after
# it).

# Once per directory that includes it: SCE_W3C_REGISTRY is a variable of the
# including directory, which a sibling directory does not see.
include_guard(DIRECTORY)

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCEW3CRegistry.cmake: SCE_CODEGEN unset. Include SCEStaticW3CTest.cmake "
        "first (it resolves the binary through SCEFindCodegen.cmake).")
endif()

# The registry, for a rule of the including directory that reads it as well
# (the C++ route generates its test registry and the visualizer's list from
# it).
set(SCE_W3C_REGISTRY "${CMAKE_SOURCE_DIR}/tests/w3c/conformance/fixtures.json")

# sce_w3c_registry_fixtures(<IDS_VAR> <HARNESS_PREFIX>)
#
#   Every fixture the registry lists, in registry order, in <IDS_VAR>, and the
#   harness each one names in <HARNESS_PREFIX><id>: `simple`, `scheduled` or
#   `http`, which the registry describes. Registry order is the order a reader
#   predicts from the file, and the order the ctest schedule and the generated
#   C++ test registry take; grouping the fixtures by harness would give the
#   same set in another order.
#
#   The configure depends on the registry, so an edit to it reconfigures the
#   tree rather than leaving a configured tree building the previous set.
#
#   A harness the registry declares and `_W3C_HARNESSES` does not iterate
#   leaves its fixtures with no harness, which refuses the configure;
#   `the_build_file_understands_every_declared_harness` says so at test time.
function(sce_w3c_registry_fixtures IDS_VAR HARNESS_PREFIX)
    if("${SCE_W3C_REGISTRY}" STREQUAL "")
        message(FATAL_ERROR
            "sce_w3c_registry_fixtures: ${CMAKE_CURRENT_SOURCE_DIR} did not include "
            "cmake/SCEW3CRegistry.cmake, which names the registry")
    endif()
    set(_W3C_REGISTRY "${SCE_W3C_REGISTRY}")
    set(_W3C_HARNESSES simple scheduled http)
    foreach(_W3C_HARNESS ${_W3C_HARNESSES})
        execute_process(
            COMMAND "${SCE_CODEGEN}" list-fixtures
                    --manifest "${_W3C_REGISTRY}"
                    --catalog w3c
                    --harness "${_W3C_HARNESS}"
                    --format cmake
            OUTPUT_VARIABLE _W3C_IDS
            OUTPUT_STRIP_TRAILING_WHITESPACE
            RESULT_VARIABLE _W3C_LIST_RESULT
            ERROR_VARIABLE _W3C_LIST_ERROR
        )
        if(NOT _W3C_LIST_RESULT EQUAL 0)
            message(FATAL_ERROR
                "sce-codegen list-fixtures --harness ${_W3C_HARNESS} failed for "
                "${_W3C_REGISTRY}: ${_W3C_LIST_ERROR}")
        endif()
        # An empty answer is a broken read, not an empty harness: the
        # registry rejects a harness no fixture names, so a silent empty
        # list here would drop a whole registration group.
        if(NOT _W3C_IDS)
            message(FATAL_ERROR
                "sce-codegen list-fixtures returned no test for harness "
                "${_W3C_HARNESS}; the registry read is broken")
        endif()
        foreach(_W3C_ID ${_W3C_IDS})
            set(_W3C_HARNESS_OF_${_W3C_ID} "${_W3C_HARNESS}")
        endforeach()
    endforeach()

    execute_process(
        COMMAND "${SCE_CODEGEN}" list-fixtures
                --manifest "${_W3C_REGISTRY}"
                --catalog w3c
                --format cmake
        OUTPUT_VARIABLE _W3C_ALL_IDS
        OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE _W3C_LIST_RESULT
        ERROR_VARIABLE _W3C_LIST_ERROR
    )
    if(NOT _W3C_LIST_RESULT EQUAL 0)
        message(FATAL_ERROR
            "sce-codegen list-fixtures failed for ${_W3C_REGISTRY}: ${_W3C_LIST_ERROR}")
    endif()
    if(NOT _W3C_ALL_IDS)
        message(FATAL_ERROR
            "sce-codegen list-fixtures returned no test at all; the registry read is broken")
    endif()
    foreach(_W3C_ID ${_W3C_ALL_IDS})
        if("${_W3C_HARNESS_OF_${_W3C_ID}}" STREQUAL "")
            message(FATAL_ERROR
                "test ${_W3C_ID} is listed by the registry but by no harness query; "
                "the two listings disagree")
        endif()
        set(${HARNESS_PREFIX}${_W3C_ID} "${_W3C_HARNESS_OF_${_W3C_ID}}" PARENT_SCOPE)
    endforeach()
    set(${IDS_VAR} "${_W3C_ALL_IDS}" PARENT_SCOPE)

    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${_W3C_REGISTRY}")
endfunction()
