# SCEBuildLoweringRun.cmake — script mode helper for SCEBuildLowering.cmake.
#
# Runs cargo, parses the JSON message stream for the `.a` cargo actually
# wrote, and leaves that path in SCE_LOWERING_PATHFILE. It is a separate
# file because `execute_process` cannot both stream a build's output to
# the developer and capture it, and because parsing belongs somewhere a
# reader can open rather than inside a quoted COMMAND line.
#
# The path comes from cargo, never from a spelled profile directory —
# see the header of SCEBuildLowering.cmake for what that rule cost once.

execute_process(
    COMMAND "${SCE_CARGO}" rustc -p sce-build --lib --release
            --crate-type staticlib --no-default-features --features ffi
            --message-format=json
    WORKING_DIRECTORY "${SCE_LOWERING_ROOT}"
    OUTPUT_VARIABLE _messages
    RESULT_VARIABLE _rc
)
if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "cargo rustc exited ${_rc}")
endif()

# One JSON object per line; the artifact is whichever `filenames` entry
# ends in `.a`. Take the LAST, because cargo emits an entry per crate in
# the graph and sce-build's own is the one that closes the build.
string(REPLACE "\n" ";" _lines "${_messages}")
set(_found "")
foreach(_line IN LISTS _lines)
    if(_line MATCHES "\"filenames\"")
        string(REGEX MATCHALL "\"[^\"]*\\.a\"" _hits "${_line}")
        foreach(_hit IN LISTS _hits)
            string(REGEX REPLACE "^\"|\"$" "" _hit "${_hit}")
            set(_found "${_hit}")
        endforeach()
    endif()
endforeach()

if(_found STREQUAL "")
    message(FATAL_ERROR
        "cargo reported no staticlib. It builds, but nothing in its JSON "
        "named a `.a` — the crate-type or the message format has moved.")
endif()

file(WRITE "${SCE_LOWERING_PATHFILE}" "${_found}")
