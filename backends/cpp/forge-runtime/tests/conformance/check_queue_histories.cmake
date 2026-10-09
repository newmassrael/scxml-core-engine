# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Judge the histories the C++ arm's stress runs wrote (SCE Protocol-Synthesis
# RFC §synth-5-P, verification layer 2) with the command every backend's
# histories are judged by.
#
#   cmake -DCODEGEN=<sce-codegen> -DDIR=<histories> -DMINIMUM=<count> \
#         -P check_queue_histories.cmake
#
# CTest runs it after `queue_history_record` (fixture `queue_histories`). The
# count is held as well as the verdicts: a recorder that wrote three of the
# histories it should have would otherwise pass, and a glob that matched
# nothing would hand the command a pattern and fail only by accident.

foreach(_required CODEGEN DIR MINIMUM)
    if(NOT DEFINED ${_required})
        message(FATAL_ERROR "check_queue_histories.cmake: -D${_required}= is required")
    endif()
endforeach()

file(GLOB _histories "${DIR}/*.json")
list(LENGTH _histories _count)
if(_count LESS MINIMUM)
    message(FATAL_ERROR
        "${_count} queue histories in ${DIR}, expected at least ${MINIMUM}: the recorder wrote fewer than it records")
endif()

execute_process(
    COMMAND "${CODEGEN}" check-queue-history ${_histories}
    RESULT_VARIABLE _rc
    OUTPUT_VARIABLE _out
    ERROR_VARIABLE _err
)

# One line per history is 160-odd lines of "ok"; only the refusals are worth
# reading, and the total says nothing was left out.
string(REGEX MATCHALL "FAIL [^\n]*" _failures "${_out}")
foreach(_failure IN LISTS _failures)
    message(STATUS "${_failure}")
endforeach()
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "queue histories refused (rc=${_rc}): ${_err}")
endif()
message(STATUS "${_count} queue histories judged, all linearizable")
