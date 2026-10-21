#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/flat_lifecycle/flat_lifecycle_sm.go and
# the package directory flat_lifecycle_transition_function/ beside it from the
# canonical fixture at integration_resources/flat_lifecycle/flat_lifecycle.scxml.
#
# Mirrors scripts/regen_flat_lifecycle.sh (Rust). The document declares its
# transition function (`sce:transition-function="emit"`,
# docs/SCE_TRANSITION_FUNCTION.md), which Go generates as a package of its own:
# a directory inside the machine's, named after it. Only `*_sm.go` and that
# directory are copied back, so the hand-authored `flat_lifecycle_test.go` next
# to the generated files is never touched.
#
# Usage (from repo root):
#   scripts/regen_flat_lifecycle_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/flat_lifecycle/flat_lifecycle.scxml"
GENERATED_DIR="backends/go/tests/integration/flat_lifecycle"
STEM="flat_lifecycle"
FUNCTION_PACKAGE="${STEM}_transition_function"
INPUT_ROOT="integration_resources/flat_lifecycle"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir, so anything the generator writes
# beside its input lands outside the tracked tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. `--input-root` overrides the default drift-header
# source-hash root so the embedded hash reflects the tracked fixture location
# instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: clear the generated files of the last run, so a renamed artefact does
# not leave the previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
rm -rf "${GENERATED_DIR:?}/$FUNCTION_PACKAGE"

# Step 5: copy the machine and the function's package back, normalizing the
# `// From:` comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go "$TMP/$FUNCTION_PACKAGE"/*.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"
cp -r "$TMP/$FUNCTION_PACKAGE" "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
