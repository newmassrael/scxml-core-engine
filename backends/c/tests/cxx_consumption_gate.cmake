# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# C++ consumption gate for the C11 backend's public and generated headers.
#
# Run in CMake script mode (`cmake -P`) by the `c11_headers_are_cxx_consumable`
# test. Every header the C11 backend publishes or generates wraps its
# declarations in `#ifdef __cplusplus extern "C"`. That wrapper states a
# contract — the same state machine an MCU compiles as C11 is linkable
# from a C++ application — and until this gate existed nothing checked it.
# It was not being met: `_Static_assert` is a C11 keyword with no C++
# spelling, and it appeared in `sce/sample.h` plus every generated
# `_sm.h`, so a C++ translation unit including any of them failed to
# parse. GCC rejected them outright; Clang accepted them as an extension
# and rejected them again under `-Werror`, which is the worse failure of
# the two because it is toolchain-dependent.
#
# What this gate compiles, and why it is one translation unit per header
# rather than one that includes them all: a header that only compiles
# when some other header was included first is not consumable on its own
# terms. Self-containedness is part of what `extern "C"` promises here,
# so each header is compiled alone.
#
# Scope is every header under the scanned roots, discovered at test time
# rather than configure time. A header added tomorrow is covered without
# anyone remembering to register it, and a root that resolves to zero
# headers fails the gate instead of passing vacuously — a silent empty
# scan is how a coverage gate turns into a coverage claim.
#
# Inputs (`-D`). Path lists use '|' as the separator, not ';': a ';' in a
# `-D` value is consumed by the shell and by CMake's own list handling
# before the script sees it, and quoting it through both is fragile
# enough that the list silently arrives as one element.
#   SCE_CXX_COMPILER   — C++ compiler to drive
#   SCE_CXX_STANDARD   — standard to compile at (e.g. 17)
#   SCE_SCAN_ROOTS     — '|'-separated dirs to scan for *.h
#   SCE_INCLUDE_DIRS   — '|'-separated dirs to place on the include path
#   SCE_WORK_DIR       — scratch dir for the generated translation units

cmake_minimum_required(VERSION 3.16)

foreach(required SCE_CXX_COMPILER SCE_CXX_STANDARD SCE_SCAN_ROOTS SCE_INCLUDE_DIRS SCE_WORK_DIR)
    if(NOT DEFINED ${required})
        message(FATAL_ERROR "cxx_consumption_gate: -D${required} is required")
    endif()
endforeach()

string(REPLACE "|" ";" SCE_SCAN_ROOTS "${SCE_SCAN_ROOTS}")
string(REPLACE "|" ";" SCE_INCLUDE_DIRS "${SCE_INCLUDE_DIRS}")

file(MAKE_DIRECTORY "${SCE_WORK_DIR}")

set(include_flags "")
foreach(dir IN LISTS SCE_INCLUDE_DIRS)
    list(APPEND include_flags "-I${dir}")
endforeach()

set(all_headers "")
set(empty_roots "")
foreach(root IN LISTS SCE_SCAN_ROOTS)
    if(NOT IS_DIRECTORY "${root}")
        list(APPEND empty_roots "${root} (not a directory)")
        continue()
    endif()
    file(GLOB_RECURSE root_headers "${root}/*.h")
    list(LENGTH root_headers root_count)
    if(root_count EQUAL 0)
        list(APPEND empty_roots "${root} (no headers)")
    else()
        message(STATUS "cxx-consumption: ${root_count} header(s) under ${root}")
        list(APPEND all_headers ${root_headers})
    endif()
endforeach()

# A root that yields nothing is a defect in the gate's wiring, not a
# pass. Reporting it as success is exactly how a gate stops covering
# what its name says it covers.
if(empty_roots)
    string(REPLACE ";" "\n  " empty_report "${empty_roots}")
    message(FATAL_ERROR
        "cxx-consumption: scan root(s) yielded no headers:\n  ${empty_report}\n"
        "The gate cannot report a pass over an empty set. If the generated\n"
        "trees are missing, build the C11 fixture targets before running ctest.")
endif()

list(LENGTH all_headers header_count)
list(SORT all_headers)

set(failures "")
set(index 0)
foreach(header IN LISTS all_headers)
    math(EXPR index "${index} + 1")
    get_filename_component(header_name "${header}" NAME)
    set(tu "${SCE_WORK_DIR}/tu_${index}_${header_name}.cpp")
    # Include by absolute path: the point is to compile this exact file,
    # not whichever same-named header the include path resolves first.
    file(WRITE "${tu}" "#include \"${header}\"\nint main() { return 0; }\n")

    execute_process(
        COMMAND "${SCE_CXX_COMPILER}"
                "-std=c++${SCE_CXX_STANDARD}"
                -fsyntax-only
                ${include_flags}
                "${tu}"
        RESULT_VARIABLE rc
        OUTPUT_VARIABLE out
        ERROR_VARIABLE err
    )
    if(NOT rc EQUAL 0)
        list(APPEND failures "${header}\n${err}")
    endif()
endforeach()

list(LENGTH failures failure_count)
if(failure_count GREATER 0)
    string(REPLACE ";" "\n---\n" failure_report "${failures}")
    message(FATAL_ERROR
        "cxx-consumption: ${failure_count} of ${header_count} header(s) do not "
        "compile as C++${SCE_CXX_STANDARD}.\n"
        "Each opens `extern \"C\"` and therefore claims to be consumable from "
        "C++.\n---\n${failure_report}")
endif()

message(STATUS
    "cxx-consumption: ${header_count} header(s) compile standalone as "
    "C++${SCE_CXX_STANDARD}")
