#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Build the codegen WASM the visualizer loads.
#
# This exists so the browser artifact has a command rather than a
# procedure. It used to be a binary committed under web/visualizer/wasm/
# that the repository could not rebuild at all: `sce-build` declared no
# cdylib crate-type, so wasm-pack refused it and the only way to produce
# one was to hand-edit Cargo.toml. The artifact consequently drifted
# from the templates it embeds and shipped a generator that failed on
# every input while every native test passed.
#
# Output is gitignored; CI (deploy-visualizer.yml) runs the same build.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${REPO_ROOT}/web/visualizer/wasm"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "error: wasm-pack not found on PATH." >&2
    echo "       install: cargo install wasm-pack" >&2
    exit 1
fi

echo "Building codegen WASM -> ${OUT_DIR}"
wasm-pack build "${REPO_ROOT}/sce-build-wasm" \
    --target web \
    --out-dir "${OUT_DIR}" \
    --out-name sce_build

echo "Done. The visualizer loads ${OUT_DIR}/sce_build.js"
