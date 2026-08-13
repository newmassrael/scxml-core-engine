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

if(NOT SCE_CODEGEN)
    get_filename_component(_SCE_FIND_CODEGEN_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)
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

# ── The generator's age ───────────────────────────────────────────────
#
# Finding the binary is not the same as finding the right one, and until
# here nothing checked the difference. CMake does not build sce-codegen:
# the generation rules list it in DEPENDS, so a *changed* binary
# regenerates, but no rule produces a new one. `target/` is also excluded
# from the transfer that populates a build machine, so a tree whose Rust
# sources are current can generate with a binary that is months old. That
# happened on 2026-08-13 — one commit compiled here and failed there, and
# the only difference between the two trees was which binary was on disk.
#
# The binary answers the question itself (`sce_build::generator_witness`
# says why the comparison must live on that side), so this asks and reports.
# The previous attempt compared mtimes from here and had to be reverted:
# the transfer is `rsync` without `-t`, so every arriving source outranked
# the remote binary and the check refused every remote configure.
#
# Only in a tree that holds the generator's sources. An installed or
# vendored SCE has nothing to compare against, and a check that cannot run
# must skip rather than fail — reporting "cannot run" as "you are stale"
# is the misattribution this repository has already paid for twice.
get_filename_component(_SCE_WITNESS_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)
if(EXISTS "${_SCE_WITNESS_ROOT}/sce-build/src" AND EXISTS "${_SCE_WITNESS_ROOT}/Cargo.lock")
    # Configure time, because configure is already a consumer: the forge
    # conformance harnesses call the generator from `execute_process` here,
    # long before the first build rule runs.
    execute_process(
        COMMAND "${SCE_CODEGEN}" verify-generator --root "${_SCE_WITNESS_ROOT}"
        RESULT_VARIABLE _sce_witness_rc
        ERROR_VARIABLE _sce_witness_err
    )
    if(NOT _sce_witness_rc EQUAL 0)
        message(FATAL_ERROR "SCE: ${_sce_witness_err}")
    endif()
    # Said out loud on success, unlike the tool itself. A check whose only
    # observable behaviour is a refusal cannot be distinguished from one
    # that is not running — which is how a gate ends up believed on a
    # machine where it silently never fired.
    message(STATUS "SCE: code generator matches the sources in ${_SCE_WITNESS_ROOT}")

    # And again at build time, which is not redundant: editing a `.rs` file
    # does not re-run configure, so without this the whole check would be
    # blind to exactly the sequence that produces the failure — edit the
    # generator, then build the C++ without rebuilding it.
    #
    # Nothing is ordered behind this target, and it does not need to be.
    # A refusal fails the build, and every generation rule already lists
    # the binary in DEPENDS — so once the generator is rebuilt, everything
    # it produced is regenerated. What a stale run leaves in the build
    # directory cannot survive into a build that succeeds.
    if(NOT TARGET sce_codegen_source_witness)
        add_custom_target(sce_codegen_source_witness ALL
            COMMAND "${SCE_CODEGEN}" verify-generator --root "${_SCE_WITNESS_ROOT}"
            COMMENT "Checking sce-codegen against the sources it was built from"
            VERBATIM
        )
    endif()
endif()
