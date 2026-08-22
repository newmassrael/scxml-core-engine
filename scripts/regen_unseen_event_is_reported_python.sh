#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/unseen_event_is_reported/*_sm.py
# from the canonical fixture at
# integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml.
#
# Mirrors scripts/regen_unseen_event_is_reported.sh (Rust). Only `*_sm.py`
# is copied back, so the hand-authored
# `test_unseen_event_is_reported_aot.py` next to it is never touched.
#
# Usage (from repo root):
#   scripts/regen_unseen_event_is_reported_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml"
GENERATED_DIR="backends/python/tests/integration/unseen_event_is_reported"
STEM="unseen_event_is_reported"
INPUT_ROOT="integration_resources/unseen_event_is_reported"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children
# land outside the canonical fixture root during the codegen run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate for any synth-invoke child.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale *_sm.py so a renamed synth-invoke does not leave the
# previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

# Step 6: copy *_sm.py in, normalizing the embedded `# From:` comment back to
# the canonical fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
