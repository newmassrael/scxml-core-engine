#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/event_data_arrives_as_sent/*_sm.py
# from the canonical fixture at
# integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml.
#
# Mirrors scripts/regen_event_data_arrives_as_sent.sh (Rust). The
# TMP-staging pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke
# children out of the canonical fixture root during the codegen run.
#
# Unlike the Rust / Kotlin / Go regen scripts, the Python generated tree is
# `.gitignored` (mirroring the W3C IRP Python pattern under
# `backends/python/tests/generated/`) — CI runs this script before pytest so
# the committed source tree never carries the SCE-GENERATED Python files.
#
# Usage (from repo root):
#   scripts/regen_event_data_arrives_as_sent_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml"
GENERATED_DIR="backends/python/tests/integration/event_data_arrives_as_sent"
STEM="event_data_arrives_as_sent"
INPUT_ROOT="integration_resources/event_data_arrives_as_sent"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

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
# synth-invoke does not leave the previous artefact orphaned.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

# Step 6: copy *_sm.py into the tracked dir + normalize the embedded
# `# From: ${TMP}/...` comment back to the canonical fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
