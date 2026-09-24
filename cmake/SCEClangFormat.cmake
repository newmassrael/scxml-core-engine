# SCEClangFormat.cmake
# How generated C++ is formatted: by sce-codegen itself, with clang-format
# pinned to major 19 (docs/SCE_CODEGEN_DETERMINISM.md §9).
#
# Provides:
#   SCE_FORMAT_GENERATED (option, default ON)
#     ON: sce-codegen formats the C++ it generates, and needs clang-format 19
#     to do it — a build without one stops with `cli/formatter-unavailable`
#     rather than compiling differently-shaped sources.
#     OFF: sce-codegen emits the templates' own bytes (`--no-format`).
#   SCE_CLANG_FORMAT_STYLE (cache path)
#     The style sce-codegen formats with. Auto-detected below; a consumer
#     may point it at their own .clang-format.
#   sce_codegen_format_args(<output_var>)
#     The sce-codegen flags that carry the two settings above, for a C++
#     generation command.
#
# This file used to run its own `clang-format -i` over generated files after
# sce-codegen wrote them, with whatever unversioned `clang-format` the
# configure found. That was a second formatter, unpinned, after the first:
# the same document then compiled from different bytes on different hosts,
# and on a host with none the pass was silently skipped. Formatting now
# happens in one place, with one pinned tool, whose identity the run manifest
# records.

option(SCE_FORMAT_GENERATED
    "Format generated C++ in sce-codegen with clang-format 19 (its default); OFF emits the templates' own bytes"
    ON)

# Default style file path — auto-detected across three distribution layouts:
#
#   In-tree:        cmake/      <-> ../tools/codegen/default.clang-format
#   Embed vendor:   embed/      <-> tools/codegen/default.clang-format (alongside)
#   System install: lib/cmake/SCE/ <-> ../../share/sce/codegen/default.clang-format
#
# Consumer-set SCE_CLANG_FORMAT_STYLE wins; auto-detection only runs when unset.
# The auto-detected file is the one sce-codegen bundles, so passing it changes
# no byte; it is passed anyway so that a consumer's override is honoured the
# same way.
if(NOT SCE_CLANG_FORMAT_STYLE)
    set(_SCE_CF_CANDIDATES
        "${CMAKE_CURRENT_LIST_DIR}/../tools/codegen/default.clang-format"
        "${CMAKE_CURRENT_LIST_DIR}/tools/codegen/default.clang-format"
        "${CMAKE_CURRENT_LIST_DIR}/../../share/sce/codegen/default.clang-format"
    )
    foreach(_sce_cf_candidate IN LISTS _SCE_CF_CANDIDATES)
        if(EXISTS "${_sce_cf_candidate}")
            set(SCE_CLANG_FORMAT_STYLE "${_sce_cf_candidate}"
                CACHE FILEPATH "clang-format style file for generated C++ code")
            break()
        endif()
    endforeach()
    unset(_SCE_CF_CANDIDATES)
endif()

# sce_codegen_format_args(<output_var>)
# Sets <output_var> to the sce-codegen flags that carry this build's
# formatting choice. Append them to a C++ generation command only: the flags
# mean nothing to the other backends, and leaving them off keeps those
# commands exactly as they were.
function(sce_codegen_format_args OUTPUT_VAR)
    if(NOT SCE_FORMAT_GENERATED)
        set(${OUTPUT_VAR} --no-format PARENT_SCOPE)
    elseif(SCE_CLANG_FORMAT_STYLE)
        set(${OUTPUT_VAR} --format-style "${SCE_CLANG_FORMAT_STYLE}" PARENT_SCOPE)
    else()
        set(${OUTPUT_VAR} "" PARENT_SCOPE)
    endif()
endfunction()

# sce_clang_format_command(<output_var> <file1> [file2 ...])
# DEPRECATED — it appended a `clang-format -i` step after generation, and
# generation now formats its own output (see the header of this file). Kept
# so a consumer build written against it still configures; it contributes no
# step.
function(sce_clang_format_command OUTPUT_VAR)
    message(DEPRECATION
        "sce_clang_format_command is deprecated and adds no step: sce-codegen "
        "formats the C++ it generates (SCE_FORMAT_GENERATED, SCE_CLANG_FORMAT_STYLE).")
    set(${OUTPUT_VAR} "" PARENT_SCOPE)
endfunction()
