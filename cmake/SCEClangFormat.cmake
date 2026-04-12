# SCEClangFormat.cmake
# Post-process generated C++ code through clang-format for consistent style.
#
# Provides:
#   sce_clang_format_files(<file1> [file2 ...])
#     Runs clang-format -i on the given files if clang-format is available.
#     Silently skips if clang-format is not found (soft dependency).
#
# Uses the project's default style from tools/codegen/default.clang-format.
# Override by setting SCE_CLANG_FORMAT_STYLE to a custom .clang-format path.

# Find clang-format (once per configure)
if(NOT DEFINED SCE_CLANG_FORMAT_FOUND)
    find_program(SCE_CLANG_FORMAT clang-format)
    if(SCE_CLANG_FORMAT)
        set(SCE_CLANG_FORMAT_FOUND TRUE CACHE BOOL "clang-format found" FORCE)
        message(STATUS "SCE: Using clang-format: ${SCE_CLANG_FORMAT}")
    else()
        set(SCE_CLANG_FORMAT_FOUND FALSE CACHE BOOL "clang-format not found" FORCE)
        message(STATUS "SCE: clang-format not found, generated C++ will use built-in formatting")
    endif()
endif()

# Default style file path
if(NOT SCE_CLANG_FORMAT_STYLE)
    set(SCE_CLANG_FORMAT_STYLE "${CMAKE_SOURCE_DIR}/tools/codegen/default.clang-format"
        CACHE FILEPATH "clang-format style file for generated C++ code")
endif()

# sce_clang_format_command(<output_var> <file1> [file2 ...])
# Appends a COMMAND clause to the given variable for use in add_custom_command.
# If clang-format is not available, appends nothing.
function(sce_clang_format_command OUTPUT_VAR)
    if(NOT SCE_CLANG_FORMAT_FOUND)
        set(${OUTPUT_VAR} "" PARENT_SCOPE)
        return()
    endif()
    set(${OUTPUT_VAR}
        COMMAND "${SCE_CLANG_FORMAT}" -style=file:${SCE_CLANG_FORMAT_STYLE} -i ${ARGN}
        PARENT_SCOPE)
endfunction()
