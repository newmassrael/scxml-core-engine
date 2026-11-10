# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Rust toolchain the emscripten C++ trees build sce-build's ECMAScript
# frontend with, in one place.
#
# The frontend is a Rust staticlib the engine links beside `lua54`
# (`cmake/SCEBuildLowering.cmake`). An emscripten tree links it for the target
# `wasm32-unknown-emscripten`, and which toolchain builds it depends on the
# tree (SCE_WASM_THREADS in the root CMakeLists.txt):
#
#   without threads  the release on PATH and its prebuilt standard library,
#                    the target being part of the release's declaration
#                    (`[platform.linux.tools.rust]` in bx.toml).
#   with threads     `-pthread` makes the link's memory shared, and wasm-ld
#                    refuses a shared memory when any object was not compiled
#                    with atomics and bulk-memory. The prebuilt standard
#                    library is, so the library is rebuilt for the target
#                    (`-Z build-std`), which only a nightly accepts, with
#                    those two features on.
#
# A floating `nightly` would turn the first language-level change of a night
# into a red wasm lane no commit of ours caused, so the nightly is pinned: it is
# the repository's one nightly, `SCE_RUST_NIGHTLY` in
# scripts/lib/rust_toolchains.sh, declared with rust-src and this target as
# `[platform.linux.tools.rust-nightly]` in bx.toml, and found where that
# declaration puts it (~/.local/rust/<nightly>). `cmake/SCEBuildLowering.cmake`
# reads the same line and builds with that directory's cargo. Moving the pin:
# change the date there, install it (`scripts/install_rust.sh --declared
# <nightly>`), then build the threaded emscripten tree.
#
# Sourced, never run.

# shellcheck source=rust_toolchains.sh
source "$(dirname "${BASH_SOURCE[0]}")/rust_toolchains.sh"
SCE_WASM_RUST_NIGHTLY="$SCE_RUST_NIGHTLY"
SCE_WASM_RUST_TARGET="wasm32-unknown-emscripten"

# Whether the standard library of the target is in the Rust on PATH. Asked of
# the compiler, not of rustup: the release the runner chose may be no rustup's.
sce_wasm_rust_release_has_target() {
    [ -d "$(rustc --print target-libdir --target "$SCE_WASM_RUST_TARGET" 2>/dev/null)" ]
}

# Whether the declared nightly is installed with what the threaded tree takes
# from it: the target's standard library and the sources `-Z build-std` builds.
sce_wasm_rust_nightly_present() {
    local dir
    sce_rust_toolchain_bin "$SCE_WASM_RUST_NIGHTLY" >/dev/null || return 1
    dir="$(sce_rust_toolchain_dir "$SCE_WASM_RUST_NIGHTLY")"
    [ -d "$dir/lib/rustlib/$SCE_WASM_RUST_TARGET/lib" ] && [ -d "$dir/lib/rustlib/src/rust/library" ]
}

# Whether what a tree with (`ON`) or without (`OFF`) threads needs is there. A
# question only: nothing is installed by asking it.
sce_wasm_rust_present() {
    sce_wasm_rust_release_has_target || return 1
    [ "$1" = "ON" ] || return 0
    sce_wasm_rust_nightly_present
}

# What is missing for a tree with (`ON`) or without (`OFF`) threads, and the
# command for each, as one sentence.
sce_wasm_rust_missing() {
    local missing=()
    sce_wasm_rust_release_has_target \
        || missing+=("the $SCE_WASM_RUST_TARGET target of $(rustc --version 2>/dev/null || echo 'the Rust on PATH') (the pinned release carries it: scripts/install_rust.sh --declared $SCE_RUST_RELEASE)")
    if [ "$1" = "ON" ] && ! sce_wasm_rust_nightly_present; then
        missing+=("Rust $SCE_WASM_RUST_NIGHTLY with rust-src and the $SCE_WASM_RUST_TARGET target at $(sce_rust_toolchain_dir "$SCE_WASM_RUST_NIGHTLY") (scripts/install_rust.sh --declared $SCE_WASM_RUST_NIGHTLY)")
    fi
    local IFS=';'
    printf '%s\n' "${missing[*]+${missing[*]}}" | sed 's/;/; /g'
}

# Install what is missing for a tree with (`ON`) or without (`OFF`) threads, on
# a hosted runner only (`sce_gate_prepare_wasm_rust` in scripts/gates/lib.sh):
# the target onto the lane's own release with the lane's rustup, and the
# declared nightly with its declared install.
sce_wasm_rust_install() {
    sce_wasm_rust_release_has_target || rustup target add "$SCE_WASM_RUST_TARGET" || return 1
    [ "$1" = "ON" ] || return 0
    sce_wasm_rust_nightly_present && return 0
    bash "$(dirname "${BASH_SOURCE[0]}")/../install_rust.sh" --declared "$SCE_WASM_RUST_NIGHTLY"
}
