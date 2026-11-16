#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/a_child_takes_a_handed_value_in_place_of_its_data/*_sm.py
# from the canonical fixture at
# integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml.
#
# The Python channel of docs/adr/0098 (§scxml-6.4: a handed value is the
# initial value of the child's top-level `<data>`, in place of its own).
# Mirrors the Rust generation of the same fixture
# (tools/codegen/trees/rust-integration.toml). The TMP-staging pattern keeps
# SCE Mesh §9.6.6 rule 1's adjacent synth-invoke children out of the
# canonical fixture root during the codegen run.
#
# The generated modules are gitignored like every other module in
# `backends/python/tests/integration/`; CI's `generate-integration -l python`
# writes the same tree before pytest.
#
# Usage (from repo root):
#   scripts/regen_a_child_takes_a_handed_value_in_place_of_its_data_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

STEM="a_child_takes_a_handed_value_in_place_of_its_data"
FIXTURE="integration_resources/$STEM/$STEM.scxml"
GENERATED_DIR="backends/python/tests/integration/$STEM"
INPUT_ROOT="integration_resources/$STEM"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so the synth-invoke children land
# outside the canonical fixture root during the codegen run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits the parent <stem>_sm.py into $TMP AND writes
# the split-out synth-invoke children (Mesh §9.6.6 rule 1) next to it.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. `--parent-stem` rewrites each child's generated
# package marker to the parent's so unqualified references to the child's
# exported symbols resolve in the shared module.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear what an earlier run generated here, so a renamed synth-invoke
# does not leave the previous module behind. The driver and the package marker
# are written by hand and are not touched.
mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Step 6: copy *_sm.py in, with the embedded `# From: ${TMP}/...` comment
# normalised back to the canonical fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
