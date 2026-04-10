#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Generate Go fixtures for the cross-language numerical conformance harness.
#
# Invokes sce-codegen on each SCXML in tests/forge/resources/ and writes the
# output into a per-package subdirectory under generated/. Each subdirectory
# is a self-contained Go package named after the fixture; the conformance
# test imports them by the module path
# github.com/newmassrael/sce-forge-runtime/conformance/generated/<name>.
#
# The per-package layout is required because Go enforces one package per
# directory, so the flat golden output under tests/forge/expected/*.go cannot
# be compiled in place.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SCE_CODEGEN="$REPO_ROOT/target/release/sce-codegen"
RESOURCE_DIR="$REPO_ROOT/tests/forge/resources"
OUT_DIR="$SCRIPT_DIR/generated"

FIXTURES=(
    filter_moving_average
    filter_debounce
    interpolation_1d_linear
    interpolation_2d_bilinear
    observer_coolant
)

if [[ ! -x "$SCE_CODEGEN" ]]; then
    echo "error: sce-codegen binary not found at $SCE_CODEGEN" >&2
    echo "  Build it first: cargo build --bin sce-codegen --features cli --release -p sce-build" >&2
    exit 1
fi

# Clean everything except the gitignore so stale fixtures cannot mask drift
find "$OUT_DIR" -mindepth 1 -not -name .gitignore -exec rm -rf {} +

for fixture in "${FIXTURES[@]}"; do
    pkg_dir="$OUT_DIR/$fixture"
    mkdir -p "$pkg_dir"
    "$SCE_CODEGEN" generate \
        "$RESOURCE_DIR/$fixture.scxml" \
        --language go \
        --output-dir "$pkg_dir/" >/dev/null
done

echo "Generated ${#FIXTURES[@]} Go fixtures under $OUT_DIR"
