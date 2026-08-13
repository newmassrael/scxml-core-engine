# SCEFindCodegen.cmake
# Single source of truth for locating the sce-codegen binary from CMake.
#
#   include(${REPO_ROOT}/cmake/SCEFindCodegen.cmake)
#   # SCE_CODEGEN now holds the resolved path
#
# Priority: 1) SCE_CODEGEN (already set by a parent CMake / installed package)
#           2) In-tree cargo build output (debug, then release)
#           3) System PATH
#
# Debug comes first because that is the only profile anything in this
# repository now builds the generator in: the binary runs identically
# either way (its cost is process start-up and I/O, not optimisation),
# while a release build compiles the whole dependency tree a second
# time instead of sharing the one clippy and the test suite already
# produced. `target/release` stays in the search path so a tree that
# still holds an older release build keeps working — but it is looked
# at second, or a stale binary would silently outrank a fresh one.
#
# Consumers include this module instead of naming a profile, because
# naming one is what broke: the profile was spelled out independently
# at ~100 sites across five languages, and moving it moved only some of
# them — the conformance jobs then looked for a release binary CI no
# longer produced. `codegen_binary_resolution.rs` fails if a profile-
# specific path reappears outside the four ecosystem locators.

# A cached path that no longer exists is worse than no path at all.
# `find_program` writes SCE_CODEGEN into CMakeCache.txt, CI restores that
# cache between runs, and `sce_codegen_drop_stale_release` deletes the
# release binary a debug-only build will not produce. The cache then names
# a file that is gone, `if(NOT SCE_CODEGEN)` is false so nothing re-resolves,
# and every generator call fails with an EMPTY error — the caller reports
# "sce-codegen list-fixtures --harness simple failed for <path>:" and stops.
# That is the C++ W3C lane's red on 2026-08-11, reproduced locally by
# configuring once and then removing the binary the cache had recorded.
if(SCE_CODEGEN AND NOT EXISTS "${SCE_CODEGEN}")
    message(STATUS
        "SCE: cached sce-codegen '${SCE_CODEGEN}' no longer exists — re-resolving")
    unset(SCE_CODEGEN CACHE)
endif()

# Resolved unconditionally: the staleness check below needs the repository root
# whether or not the binary itself had to be searched for, and a cached
# SCE_CODEGEN skips the search entirely — which is the common case, and exactly
# the case where a stale binary persists.
get_filename_component(_SCE_FIND_CODEGEN_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)

if(NOT SCE_CODEGEN)
    find_program(SCE_CODEGEN sce-codegen
        PATHS "${_SCE_FIND_CODEGEN_ROOT}/target/debug"
              "${_SCE_FIND_CODEGEN_ROOT}/target/release"
        NO_DEFAULT_PATH
    )
    if(NOT SCE_CODEGEN)
        find_program(SCE_CODEGEN sce-codegen)
    endif()
endif()

if(NOT SCE_CODEGEN)
    message(FATAL_ERROR
        "SCE: sce-codegen not found. Build it with: "
        "cargo build --bin sce-codegen --features cli -p sce-build")
endif()

# A generator older than its own sources produces stale code, silently.
#
# Nothing here builds sce-codegen: CMake resolves whichever binary happens to
# sit in target/, and `target/` is excluded from the rsync that seeds a build
# machine, so a tree whose Rust sources are current can still regenerate from
# a binary months behind them. On 2026-08-13 that turned a fixed parser into a
# build failure on the build machine while the same commit compiled locally —
# the two differed in nothing but which binary was on disk.
#
# A build error is the lucky outcome. The same staleness in the other
# direction — an older generator that accepts more — emits code that compiles
# and is wrong, which is exactly the class the acceptance tests cannot see
# because they run against the generator, not against its age.
#
# Only meaningful in-tree; an installed package ships the binary without the
# sources it was built from.
if(EXISTS "${_SCE_FIND_CODEGEN_ROOT}/sce-build/src")
    file(GLOB_RECURSE _SCE_CODEGEN_SOURCES CONFIGURE_DEPENDS
        "${_SCE_FIND_CODEGEN_ROOT}/sce-build/src/*.rs"
        "${_SCE_FIND_CODEGEN_ROOT}/tools/codegen/templates/*")

    set(_SCE_CODEGEN_STALE_AFTER "")
    foreach(_sce_src IN LISTS _SCE_CODEGEN_SOURCES)
        if("${_sce_src}" IS_NEWER_THAN "${SCE_CODEGEN}")
            set(_SCE_CODEGEN_STALE_AFTER "${_sce_src}")
            break()
        endif()
    endforeach()

    if(_SCE_CODEGEN_STALE_AFTER)
        message(FATAL_ERROR
            "SCE: sce-codegen is older than its own sources, so every "
            "generated file this build produces would come from a stale "
            "generator.\n"
            "  generator: ${SCE_CODEGEN}\n"
            "  newer:     ${_SCE_CODEGEN_STALE_AFTER}\n"
            "Rebuild it with: "
            "cargo build --bin sce-codegen --features cli -p sce-build")
    endif()
endif()
