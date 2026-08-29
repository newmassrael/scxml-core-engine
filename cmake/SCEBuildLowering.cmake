# SCEBuildLowering.cmake
# Single source of truth for building and linking sce-build's C lowering
# surface into the C++ engine.
#
#   include(${REPO_ROOT}/cmake/SCEBuildLowering.cmake)
#   # target `SCE::Lowering` now exists
#
# ── Why this builds rather than finds ─────────────────────────────────
#
# `SCEFindCodegen.cmake` FINDS its artifact, because every lane already
# builds `sce-codegen` as an explicit step before configuring. Nothing
# builds this one, and adding a step to the eight workflows that build a
# C++ target is eight places to drift — the same shape as the ~100
# hand-spelled profile paths that module's header records. Cargo is
# present wherever a C++ target is built (every such workflow runs
# `dtolnay/rust-toolchain@stable` before CMake), so building here costs
# one place instead of eight.
#
# ── Why a staticlib ───────────────────────────────────────────────────
#
# A cdylib would need an RPATH, an install rule and a copy beside every
# test binary before anything could run. A staticlib links like `lua54`
# does and leaves no runtime artifact to locate. The D1 ledger's size row
# was measured on a cdylib because that is the right instrument for "how
# much lowering code is reachable"; what an image PAYS for the shape
# actually chosen is measured separately and is larger — see
# `scripts/measure-lowering-footprint.sh` and the ledger's
# `link-beside-lua` row.
#
# ── Ask cargo where it put the artifact ───────────────────────────────
#
# Not `${CMAKE_SOURCE_DIR}/target/release/libsce_build.a`. Spelling a
# profile directory is what `codegen_binary_resolution` exists to forbid:
# a second copy of the path is a second copy of the profile ORDER, and
# moving the profile moved only some of them. `--message-format=json`
# makes cargo answer.

if(TARGET SCE::Lowering)
    return()
endif()

get_filename_component(_SCE_LOWERING_ROOT "${CMAKE_CURRENT_LIST_DIR}" DIRECTORY)

find_program(SCE_CARGO cargo)
if(NOT SCE_CARGO)
    message(FATAL_ERROR
        "SCE: cargo not found, and sce_scripting links sce-build's lowering "
        "surface. Install a Rust toolchain, or configure with "
        "-DSCE_ENABLE_LUA=OFF.")
endif()

# Release, deliberately, and unlike the generator. The generator's cost is
# process start-up, so a debug build of it is free; this one is compiled
# INTO the engine and its per-call cost is the number the D1 ledger
# reports (577ns parsed against 1085ns rewritten). A debug build would
# make the engine slower than the rewriter it is replacing and the
# comparison meaningless.
set(_SCE_LOWERING_STAMP "${CMAKE_BINARY_DIR}/sce_lowering_build.stamp")
set(_SCE_LOWERING_PATHFILE "${CMAKE_BINARY_DIR}/sce_lowering_path.txt")

# `--no-default-features` drops `xsd`, whose libxml2 the lowering entry
# points cannot reach. Keeping it would make every C++ consumer inherit a
# C library for code the linker discards.
execute_process(
    COMMAND "${CMAKE_COMMAND}"
            -DSCE_CARGO=${SCE_CARGO}
            -DSCE_LOWERING_ROOT=${_SCE_LOWERING_ROOT}
            -DSCE_LOWERING_PATHFILE=${_SCE_LOWERING_PATHFILE}
            -P "${CMAKE_CURRENT_LIST_DIR}/SCEBuildLoweringRun.cmake"
    RESULT_VARIABLE _sce_lowering_rc
)
if(NOT _sce_lowering_rc EQUAL 0)
    message(FATAL_ERROR
        "SCE: building sce-build's lowering staticlib failed (${_sce_lowering_rc}). "
        "Reproduce with: cargo rustc -p sce-build --lib --release "
        "--crate-type staticlib --no-default-features --features ffi")
endif()

file(READ "${_SCE_LOWERING_PATHFILE}" SCE_LOWERING_LIB)
string(STRIP "${SCE_LOWERING_LIB}" SCE_LOWERING_LIB)
if(NOT EXISTS "${SCE_LOWERING_LIB}")
    message(FATAL_ERROR
        "SCE: cargo reported '${SCE_LOWERING_LIB}' for the lowering staticlib "
        "and it does not exist.")
endif()

add_library(SCE::Lowering STATIC IMPORTED GLOBAL)
set_target_properties(SCE::Lowering PROPERTIES
    IMPORTED_LOCATION "${SCE_LOWERING_LIB}"
    INTERFACE_INCLUDE_DIRECTORIES "${_SCE_LOWERING_ROOT}/sce/include"
)
# What a Rust staticlib needs from the platform. Rust's std uses threads,
# `dlsym` for backtrace symbolisation, and libm.
set_property(TARGET SCE::Lowering PROPERTY
    INTERFACE_LINK_LIBRARIES pthread dl m)

message(STATUS "SCE: lowering surface -> ${SCE_LOWERING_LIB}")
