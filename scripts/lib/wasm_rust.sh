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
#   without threads  the stable toolchain and its prebuilt standard library.
#   with threads     `-pthread` makes the link's memory shared, and wasm-ld
#                    refuses a shared memory when any object was not compiled
#                    with atomics and bulk-memory. The prebuilt standard
#                    library is, so the library is rebuilt for the target
#                    (`-Z build-std`), which only a nightly accepts, with
#                    those two features on.
#
# A floating `nightly` would turn the first language-level change of a night
# into a red wasm lane no commit of ours caused, so the nightly is pinned. Two
# things read the pin: `cmake/SCEBuildLowering.cmake`, which parses the line
# below (so it stays a plain `NAME="value"` assignment), and the CI lanes that
# install the toolchain through `sce_wasm_rust_install`. A date spelled in each
# would be two answers to one question.
#
# Sourced, never run. Moving the pin: change the date, run
# `sce_wasm_rust_install` once, then the threaded emscripten tree's build.
SCE_WASM_RUST_NIGHTLY="nightly-2026-10-08"

# Install what building the frontend for the emscripten target needs: the
# target for the stable toolchain (the tree without threads) and, for the
# pinned nightly, the target plus the standard library's sources that
# `-Z build-std` compiles (the tree with threads). Idempotent.
sce_wasm_rust_install() {
    rustup target add wasm32-unknown-emscripten || return 1
    rustup toolchain install "${SCE_WASM_RUST_NIGHTLY}" --profile minimal \
        --component rust-src --target wasm32-unknown-emscripten || return 1
}
