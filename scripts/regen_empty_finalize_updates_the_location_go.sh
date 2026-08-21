#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/empty_finalize_updates_the_location/
# from the canonical fixture at
# integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml.
#
# Mirrors scripts/regen_empty_finalize_updates_the_location.sh (Rust). Only
# `*_sm.go` is copied back, so the hand-authored `*_test.go` is untouched.
#
# Usage (from repo root):
#   scripts/regen_empty_finalize_updates_the_location_go.sh

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml"
GENERATED_DIR="backends/go/tests/integration/empty_finalize_updates_the_location"
STEM="empty_finalize_updates_the_location"
INPUT_ROOT="integration_resources/empty_finalize_updates_the_location"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
