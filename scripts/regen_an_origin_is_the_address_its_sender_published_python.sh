#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/an_origin_is_the_address_its_sender_published/*_sm.py
# from the canonical fixture at
# integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml.
#
# The TMP-staging pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke
# child (the inline `kid`) out of the canonical fixture root during the codegen
# run. As with the other Python integration regens, the generated tree is
# `.gitignored` and CI runs this script (through `generate-integration -l python`)
# before pytest.
#
# Usage (from repo root):
#   scripts/regen_an_origin_is_the_address_its_sender_published_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml"
GENERATED_DIR="backends/python/tests/integration/an_origin_is_the_address_its_sender_published"
STEM="an_origin_is_the_address_its_sender_published"
INPUT_ROOT="integration_resources/an_origin_is_the_address_its_sender_published"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# The parent, and the synth-invoke child it splits out beside it.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Each child, with `--parent-stem` so its generated module resolves beside the
# parent's.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the embedded `# From: ${TMP}/...` comment back to the canonical
# fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
