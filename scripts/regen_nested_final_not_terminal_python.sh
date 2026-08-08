#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/nested_final_not_terminal/*_sm.py
# from the canonical fixture at
# integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml.
#
# Mirrors scripts/regen_nested_final_not_terminal.sh (Rust). The TMP-staging
# pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke children
# (sce-build/src/parser.rs:1804-1805) out of the canonical fixture root
# during the codegen run.
#
# Unlike the Rust / Kotlin / Go regen scripts, the Python generated
# tree is `.gitignored` (mirroring the W3C IRP Python pattern under
# `backends/python/tests/generated/`) — CI runs this script before pytest
# so the committed source tree never carries the SCE-GENERATED Python
# files. The pybind11 channel test at `backends/python/bindings/tests/test_nested_final_not_terminal.py`
# stays untouched (dual-channel: pybind11 → C++ Interpreter +
# Python AOT codegen).
#
# Usage (from repo root):
#   scripts/regen_nested_final_not_terminal_python.sh
#
# Requires:
#   target/debug/sce-codegen (build first if missing: see step 1).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/debug/sce-codegen"
FIXTURE="integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml"
GENERATED_DIR="backends/python/tests/integration/nested_final_not_terminal"
STEM="nested_final_not_terminal"
INPUT_ROOT="integration_resources/nested_final_not_terminal"

# Step 1: build sce-codegen in release mode if absent.
if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli -p sce-build
fi

# Step 2: stage the fixture into a tmp dir so synth-invoke children
# land outside the canonical fixture root during the codegen run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits parent <stem>_sm.py into $TMP AND
# writes the split-out synth-invoke children (Mesh §9.6.6 rule 1)
# into $TMP next to the staged parent.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# generated package marker to the parent's so unqualified references
# to the child's exported symbols resolve in the shared module.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale *_sm.py in the tracked output dir so a renamed
# synth-invoke (e.g. when the fixture's invoke `id` changes) does not
# leave the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

# Step 6: copy *_sm.py into the tracked dir + normalize the embedded
# `# From: ${TMP}/...` comment back to the canonical fixture path so
# the committed (read-only on CI, but local-tracked nonetheless)
# header advertises a stable path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
