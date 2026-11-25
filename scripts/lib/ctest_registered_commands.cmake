# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Every test a configured tree registers and the command it runs, one per line
# on standard error as `name<TAB>argument<TAB>argument...`, read from the
# tree's `CTestTestfile.cmake` files:
#
#   cmake -DSCE_CTEST_TREE=<build dir> -P scripts/lib/ctest_registered_commands.cmake
#
# Because ctest will not say it: `--show-only=json-v1` leaves `command` out of
# a test whose executable does not exist yet, which in a tree that is
# configured and not built is every test. Measured 2026-10-26 on run
# 40257956731: the mutation harness's build closure came back empty for every
# such tree, and the first build of every round in its lane was the whole tree.
#
# CMake reads its own language here, with the three commands those files are
# made of (`add_test`, `set_tests_properties`, `subdirs`) defined to report
# instead of to register. A file that calls anything else stops the reading
# with an error, and a caller takes that as "the commands cannot be named".
#
# Two readers: the mutation harness (`mutation_ctest_registered_commands` in
# scripts/mutate), which builds only what a round's tests run, and
# scripts/lib/ctest_shards.py, which builds only what one shard of the C++
# suite runs.
function(add_test name)
  string(REPLACE ";" "\t" command "${ARGN}")
  message("${name}\t${command}")
endfunction()
function(set_tests_properties)
endfunction()
function(subdirs)
  foreach(dir IN LISTS ARGN)
    if(IS_ABSOLUTE "${dir}")
      set(file "${dir}/CTestTestfile.cmake")
    else()
      set(file "${CMAKE_CURRENT_LIST_DIR}/${dir}/CTestTestfile.cmake")
    endif()
    if(EXISTS "${file}")
      include("${file}")
    endif()
  endforeach()
endfunction()
if(NOT SCE_CTEST_TREE)
  message(FATAL_ERROR "ctest_registered_commands.cmake: pass -DSCE_CTEST_TREE=<build dir>")
endif()
include("${SCE_CTEST_TREE}/CTestTestfile.cmake")
