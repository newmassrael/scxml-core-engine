# SCEForceInclude.cmake
# A header read ahead of every source file of a target, in the spelling of the
# command line that compiles it.
#
#   include(${CMAKE_SOURCE_DIR}/cmake/SCEForceInclude.cmake)
#   sce_force_include(<target> <PRIVATE|PUBLIC|INTERFACE> <header>...)
#
# GCC and Clang take `-include <header>`; MSVC takes `/FI<header>`, and so does
# clang-cl, whose command line is cl's. CMake names that command line, not the
# compiler, with its MSVC variable (true for cl and for clang-cl alike), so the
# spelling follows it: the compiler id alone would hand clang-cl the GCC form.
# Either way the header is found as `#include "<header>"` finds it, so a name
# relative to an include directory of the target is enough.
#
# The GCC form is passed through SHELL: so that `-include` and its argument stay
# two tokens and a second `-include` is not dropped as a repeat of the first,
# which target_compile_options does to an option it has seen. The argument is
# quoted inside it, so a path with a space stays one token.
include_guard(GLOBAL)

function(sce_force_include target scope)
    foreach(_sce_force_included IN LISTS ARGN)
        if(MSVC)
            target_compile_options(${target} ${scope} "/FI${_sce_force_included}")
        else()
            target_compile_options(${target} ${scope} "SHELL:-include \"${_sce_force_included}\"")
        endif()
    endforeach()
endfunction()
