#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/nested_lifecycle/{nested_lifecycle_sm,
# nested_lifecycle_transition_function}.py from the canonical fixture at
# integration_resources/nested_lifecycle/nested_lifecycle.scxml.
#
# Mirrors scripts/regen_nested_lifecycle.sh (Rust). The document declares its
# transition function (`sce:transition-function="emit"`,
# docs/SCE_TRANSITION_FUNCTION.md), which Python generates as a module of its own
# beside the machine's.
#
# Unlike the Rust / Kotlin / Go regen scripts, the Python generated tree is
# `.gitignored` (mirroring the W3C IRP Python pattern under
# `backends/python/tests/generated/`) — CI runs this script before pytest so the
# committed source tree never carries the SCE-GENERATED Python files.
#
# Usage (from repo root):
#   scripts/regen_nested_lifecycle_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/nested_lifecycle/nested_lifecycle.scxml"
GENERATED_DIR="backends/python/tests/integration/nested_lifecycle"
STEM="nested_lifecycle"
INPUT_ROOT="integration_resources/nested_lifecycle"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir, so anything the generator writes
# beside its input lands outside the canonical fixture root.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate the machine module and the function module.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: clear the generated modules of the last run, so a renamed artefact
# does not leave the previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Step 5: copy both modules into the tracked dir and normalize the embedded
# `# From: ${TMP}/...` comment back to the canonical fixture path.
for src in "$TMP"/*_sm.py "$TMP"/*_transition_function.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$TMP"/*_transition_function.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
