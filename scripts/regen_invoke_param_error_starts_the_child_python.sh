#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/invoke_param_error_starts_the_child/
# from the canonical fixture at
# integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml.
#
# Mirrors scripts/regen_invoke_param_error_starts_the_child.sh (Rust).
#
# Usage (from repo root):
#   scripts/regen_invoke_param_error_starts_the_child_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml"
GENERATED_DIR="backends/python/tests/integration/invoke_param_error_starts_the_child"
STEM="invoke_param_error_starts_the_child"
INPUT_ROOT="integration_resources/invoke_param_error_starts_the_child"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
