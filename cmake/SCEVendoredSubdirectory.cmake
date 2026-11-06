# SCEVendoredSubdirectory.cmake
# A vendored upstream project, added with its own CMakeLists.txt, is built with
# the warning flags its authors chose, not with this repository's.
#
#   include(${CMAKE_SOURCE_DIR}/cmake/SCEVendoredSubdirectory.cmake)
#   sce_add_vendored_subdirectory(<source_dir> <binary_dir> [EXCLUDE_FROM_ALL] [SYSTEM])
#
# Why. The top-level CMakeLists.txt holds this repository's code to
# `-Wall -Wextra -pedantic -Werror` (`/W4 /WX` on MSVC) with add_compile_options,
# and a directory property reaches every directory added below it. A vendored
# project therefore compiled under -Werror with warning classes its authors never
# build with, and any new compiler or C library could stop the build in code
# this repository does not own. That happened on 2026-11-06: on GCC 15.2 and
# glibc 2.43, QuickJS's `p = strrchr(base_name, '/')` became a
# -Wdiscarded-qualifiers error, and QuickJS-ng v0.13.0, which fixes it, still
# failed SCE's -pedantic on `return` of a void expression. Upstream itself makes
# -Werror opt-in (QJS_BUILD_WERROR, off by default).
#
# The project's own code keeps every flag: only the directory added here starts
# from no inherited compile options, and the caller's directory gets its options
# back at once, so the targets it defines after the call are unchanged. Compile
# definitions and include directories are inherited as before.
#
# A third-party tree this repository wrote the CMakeLists.txt for (third_party/lua)
# is not vendored in this sense: its build is this repository's, and it keeps
# this repository's flags.
include_guard(GLOBAL)

function(sce_add_vendored_subdirectory source_dir binary_dir)
    get_directory_property(sce_project_compile_options COMPILE_OPTIONS)
    set_directory_properties(PROPERTIES COMPILE_OPTIONS "")
    add_subdirectory("${source_dir}" "${binary_dir}" ${ARGN})
    set_directory_properties(PROPERTIES COMPILE_OPTIONS "${sce_project_compile_options}")
endfunction()
