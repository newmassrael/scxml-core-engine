# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Judge the histories a runtime's inbox runs wrote (docs/adr/0147, D3) with the
# command every runtime's inbox histories are judged by, against the promise
# docs/formal/InboxService.tla states. Shared by the runtimes whose build is
# CMake (C11, and the generated C++ engine when it writes its own), so they read
# one script and not copies of it. It is check_queue_histories.cmake for the
# other command and the other format.
#
#   cmake -DCODEGEN=<sce-codegen> -DDIR=<histories> -DMINIMUM=<count> \
#         -P check_inbox_histories.cmake
#
# CTest runs it after the program that records the runs (a fixture named for
# the runtime). The count is held as well as the verdicts: a recorder that wrote
# three of the histories it should have would otherwise pass, and a glob that
# matched nothing would hand the command a pattern and fail only by accident.

foreach(_required CODEGEN DIR MINIMUM)
    if(NOT DEFINED ${_required})
        message(FATAL_ERROR "check_inbox_histories.cmake: -D${_required}= is required")
    endif()
endforeach()

file(GLOB _histories "${DIR}/*.json")
list(LENGTH _histories _count)
if(_count LESS MINIMUM)
    message(FATAL_ERROR
        "${_count} inbox histories in ${DIR}, expected at least ${MINIMUM}: the recorder wrote fewer than it records")
endif()

execute_process(
    COMMAND "${CODEGEN}" check-inbox-history ${_histories}
    RESULT_VARIABLE _rc
    OUTPUT_VARIABLE _out
    ERROR_VARIABLE _err
)

# One line per history is a line of "ok" each; only the refusals are worth
# reading, and the total says nothing was left out.
string(REGEX MATCHALL "FAIL [^\n]*" _failures "${_out}")
foreach(_failure IN LISTS _failures)
    message(STATUS "${_failure}")
endforeach()
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "inbox histories refused (rc=${_rc}): ${_err}")
endif()
message(STATUS "${_count} inbox histories judged, all linearizable")
