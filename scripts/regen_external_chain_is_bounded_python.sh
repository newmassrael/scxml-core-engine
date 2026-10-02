#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/external_chain_is_bounded/*_sm.py
# from tests/integration/external_chain_is_bounded.scxml.
#
# The document is not under `integration_resources/`: a stem there is a
# seven-channel contract and `generate-integration` fans it out to every
# language, while the engines other than Python do not hold this budget yet
# (ARCHITECTURE.md "External-Event Budget"). It moves under it when each
# channel has its driver, as `a_child_timer_is_a_deadline_of_its_parent` would.
# The generated modules are gitignored like every other module in
# `backends/python/tests/integration/`; the `w3c-python` gate regenerates them,
# and this script is for a local run.
#
# Usage (from repo root):
#   scripts/regen_external_chain_is_bounded_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="tests/integration/external_chain_is_bounded.scxml"
GENERATED_DIR="backends/python/tests/integration/external_chain_is_bounded"
STEM="external_chain_is_bounded"
INPUT_ROOT="tests/integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so the generator writes outside the
# canonical fixture root.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: clear stale *_sm.py in the output dir, then copy the new one in with
# the embedded `# From: ${TMP}/...` comment normalized back to the canonical
# fixture path.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
