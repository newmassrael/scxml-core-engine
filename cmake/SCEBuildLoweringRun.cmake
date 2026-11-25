# SCEBuildLoweringRun.cmake — script mode helper for SCEBuildLowering.cmake.
#
# Runs cargo, reads its JSON message stream for the static library cargo
# actually wrote and for the native libraries rustc says that library needs,
# and leaves the path in SCE_LOWERING_PATHFILE and the libraries in
# SCE_LOWERING_NATIVELIBSFILE. It is a separate file because
# `execute_process` cannot both stream a build's output to the developer and
# capture it, and because parsing belongs somewhere a reader can open rather
# than inside a quoted COMMAND line.
#
# The stream is read with CMake's JSON parser, not a pattern: a path in it is
# a JSON string, so on Windows its backslashes arrive escaped, and only a
# parser gives the path back as the file system spells it. The static
# library is recognised by SCE_LOWERING_STATICLIB_SUFFIX, the suffix the
# including tree's toolchain gives one (`.a` for GNU and emscripten, `.lib`
# for MSVC), so no platform's spelling is written here.
#
# The path comes from cargo, never from a spelled profile directory —
# see the header of SCEBuildLowering.cmake for what that rule cost once.

#
# What differs by tree is only how cargo is asked, and it is decided by the
# variables the including module passes:
#
#   SCE_LOWERING_TARGET      the rustc target; empty builds for the host.
#   SCE_LOWERING_TARGET_DIR  where cargo builds; empty is the workspace's own
#                            `target/`. A tree that is not the host's gets a
#                            directory of its own, because two trees that
#                            differ in RUSTFLAGS and in `-Z build-std` sharing
#                            one would each rebuild the other's artifact on
#                            every build.
#   SCE_LOWERING_NIGHTLY_BIN the `bin/` of the pinned nightly toolchain;
#                            non-empty builds with its cargo, its `bin/` first
#                            on PATH so that rustc is its own too, and rebuilds
#                            the standard library with atomics and bulk-memory
#                            (scripts/lib/wasm_rust.sh says why a threaded
#                            emscripten link needs that).
set(_cargo_env "")
set(_cargo_args rustc -p sce-build --lib --release
    --crate-type staticlib --no-default-features --features ffi
    --message-format=json)
if(SCE_LOWERING_TARGET)
    list(APPEND _cargo_args --target "${SCE_LOWERING_TARGET}")
endif()
# What a Rust static library needs linked beside it differs by target (the
# C library, threads and the unwinder on Linux; kernel32, ws2_32, bcrypt and
# the rest on Windows), and rustc is the one that knows. It reports the list
# for the target it built; a list written here would be one platform's.
#
# These are rustc's arguments, which cargo takes after a `--` that ends its
# own. They are kept in a list of their own and placed last when the command
# is put together, so that a cargo argument added below (the nightly's
# `-Z build-std`) stays cargo's: appended after the `--` it reached rustc,
# which refused it as an option of its own, and the threaded emscripten tree
# could not build the library at all.
set(_rustc_args --print native-static-libs)
if(SCE_LOWERING_TARGET_DIR)
    list(APPEND _cargo_env "CARGO_TARGET_DIR=${SCE_LOWERING_TARGET_DIR}")
endif()
if(SCE_LOWERING_NIGHTLY_BIN)
    set(SCE_CARGO "${SCE_LOWERING_NIGHTLY_BIN}/cargo")
    # RUSTFLAGS replaces `build.rustflags`, so the flags a build was given there
    # (CARGO_BUILD_RUSTFLAGS: the mapping of Cargo's home that
    # scripts/lib/reproducible_paths.sh sets) are carried over into it.
    list(APPEND _cargo_env
        "PATH=${SCE_LOWERING_NIGHTLY_BIN}:$ENV{PATH}"
        "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals $ENV{CARGO_BUILD_RUSTFLAGS}")
    list(APPEND _cargo_args -Z build-std=std,panic_abort)
endif()

execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env ${_cargo_env} "${SCE_CARGO}" ${_cargo_args}
            -- ${_rustc_args}
    WORKING_DIRECTORY "${SCE_LOWERING_ROOT}"
    OUTPUT_VARIABLE _messages
    RESULT_VARIABLE _rc
)
if(SCE_LOWERING_STATICLIB_SUFFIX STREQUAL "")
    message(FATAL_ERROR "SCE_LOWERING_STATICLIB_SUFFIX was not passed")
endif()

# One JSON object per line. The artifact is whichever `filenames` entry of a
# `compiler-artifact` ends in the static library suffix; take the LAST,
# because cargo emits an entry per crate in the graph and sce-build's own is
# the one that closes the build. rustc's list of native libraries arrives as
# a `compiler-message` whose text starts `native-static-libs:`.
#
# When cargo fails, the compiler's diagnostics are in this stream too and
# nowhere else: `--message-format=json` puts them on stdout, which
# OUTPUT_VARIABLE holds, so cargo's stderr says only that a crate "could not
# compile". Each carries the text rustc would have printed as
# `message.rendered`, and those are printed, so a failed build says why in
# the log of the tree that ran it.
#
# The lines are walked by the position of each newline, not as a CMake list:
# a rendered diagnostic holds `;` and `[`, and a list split at the one and
# joined across the other would hand the JSON parser half an object, which
# drops that diagnostic and any artifact on the lines it swallowed.
set(_found "")
set(_native "")
set(_native_seen FALSE)
set(_native_prefix "native-static-libs:")
string(LENGTH "${_native_prefix}" _native_prefix_len)
string(LENGTH "${SCE_LOWERING_STATICLIB_SUFFIX}" _suffix_len)
set(_rest "${_messages}")
while(NOT _rest STREQUAL "")
    string(FIND "${_rest}" "\n" _nl)
    if(_nl EQUAL -1)
        set(_line "${_rest}")
        set(_rest "")
    else()
        string(SUBSTRING "${_rest}" 0 ${_nl} _line)
        math(EXPR _nl "${_nl} + 1")
        string(SUBSTRING "${_rest}" ${_nl} -1 _rest)
    endif()
    if(NOT _line MATCHES "^{")
        continue()
    endif()
    string(JSON _reason ERROR_VARIABLE _err GET "${_line}" reason)
    if(_err)
        continue()
    endif()
    if(_reason STREQUAL "compiler-artifact")
        string(JSON _count ERROR_VARIABLE _err LENGTH "${_line}" filenames)
        if(_err OR _count EQUAL 0)
            continue()
        endif()
        math(EXPR _last "${_count} - 1")
        foreach(_i RANGE 0 ${_last})
            string(JSON _file GET "${_line}" filenames ${_i})
            string(LENGTH "${_file}" _file_len)
            if(_file_len GREATER _suffix_len)
                math(EXPR _at "${_file_len} - ${_suffix_len}")
                string(SUBSTRING "${_file}" ${_at} -1 _tail)
                if(_tail STREQUAL SCE_LOWERING_STATICLIB_SUFFIX)
                    set(_found "${_file}")
                endif()
            endif()
        endforeach()
    elseif(_reason STREQUAL "compiler-message")
        if(NOT _rc EQUAL 0)
            string(JSON _rendered ERROR_VARIABLE _err GET "${_line}" message rendered)
            if(NOT _err AND NOT _rendered STREQUAL "")
                message("${_rendered}")
            endif()
            continue()
        endif()
        string(JSON _text ERROR_VARIABLE _err GET "${_line}" message message)
        if(_err)
            continue()
        endif()
        string(FIND "${_text}" "${_native_prefix}" _at)
        if(_at EQUAL 0)
            string(SUBSTRING "${_text}" ${_native_prefix_len} -1 _native)
            string(STRIP "${_native}" _native)
            set(_native_seen TRUE)
        endif()
    endif()
endwhile()

if(NOT _rc EQUAL 0)
    message(FATAL_ERROR "cargo rustc exited ${_rc}; rustc's diagnostics are above")
endif()

if(_found STREQUAL "")
    message(FATAL_ERROR
        "cargo reported no staticlib. It builds, but nothing in its JSON named "
        "a file ending in `${SCE_LOWERING_STATICLIB_SUFFIX}`: the crate-type or "
        "the message format has moved.")
endif()
if(NOT _native_seen)
    message(FATAL_ERROR
        "rustc reported no `native-static-libs` for the staticlib, so what it "
        "needs linked beside it is unknown: `--print native-static-libs` or "
        "its message has moved.")
endif()

file(WRITE "${SCE_LOWERING_PATHFILE}" "${_found}")
file(WRITE "${SCE_LOWERING_NATIVELIBSFILE}" "${_native}")
