#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity Atomic γ-3 (go half) — regenerate the
# 3 default-marker fixtures used by the round-trip property test
# (default_round_trip_test.go). Mirrors the conformance harness'
# generate.sh shape (sce-forge-runtime/go/conformance/generate.sh) but
# narrowed to the RFC-dedicated fixtures, which live outside the
# numerical-conformance manifest because their purpose is contract
# testing (Default emission), not oracle comparison.
#
# Run manually after editing the SCXML fixtures or any of the go/
# codec templates:
#
#   sce-forge-runtime/go/round_trip/generate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SCE_CODEGEN="$REPO_ROOT/target/release/sce-codegen"
RESOURCE_DIR="$REPO_ROOT/tests/forge/resources"
OUT_DIR="$SCRIPT_DIR/generated"

GO_MOD_FILE="$REPO_ROOT/sce-forge-runtime/go/go.mod"
GO_MODULE_ROOT="$(awk '$1 == "module" { print $2; exit }' "$GO_MOD_FILE")"
GO_MODULE_PREFIX="$GO_MODULE_ROOT/round_trip/generated"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli --release -p sce-build)
fi

if [[ ! -x "$SCE_CODEGEN" ]]; then
    echo "error: sce-codegen binary not found at $SCE_CODEGEN" >&2
    echo "  Build it first: cargo build --bin sce-codegen --features cli --release -p sce-build" >&2
    exit 1
fi

# Clean stale fixtures (keep the .gitignore-equivalent marker).
find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true

FIXTURES=(
    codec_default_marker_arm_a
    codec_default_marker_arm_b
    codec_variant_default_marker
)

for fixture in "${FIXTURES[@]}"; do
    pkg_dir="$OUT_DIR/$fixture"
    mkdir -p "$pkg_dir"
    "$SCE_CODEGEN" generate \
        "$RESOURCE_DIR/$fixture.scxml" \
        --language go \
        --output-dir "$pkg_dir/" \
        --go-module-prefix "$GO_MODULE_PREFIX" >/dev/null
done

echo "Generated ${#FIXTURES[@]} Go fixtures under $OUT_DIR"
(cd "$REPO_ROOT/sce-forge-runtime/go" && go build ./round_trip/...)
echo "go build ./round_trip/... OK"
