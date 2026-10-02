#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/external_chain_is_bounded/*_sm.go
# from tests/integration/external_chain_is_bounded.scxml.
#
# Mirrors scripts/regen_external_chain_is_bounded.sh (Rust), which says why the
# document is not under `integration_resources/`. Only `*_sm.go` is copied
# back, so the hand-authored `*_test.go` next to the generated file is never
# touched.
#
# Usage (from repo root):
#   scripts/regen_external_chain_is_bounded_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="tests/integration/external_chain_is_bounded.scxml"
GENERATED_DIR="backends/go/tests/integration/external_chain_is_bounded"
STEM="external_chain_is_bounded"
INPUT_ROOT="tests/integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so anything the generator writes
# beside its input lands outside the tracked tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. `--input-root` overrides the default drift-header
# source-hash root so the embedded hash reflects the tracked fixture location
# instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: clear stale `*_sm.go` files so a renamed artefact does not leave the
# previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Step 5: copy only the `*_sm.go` artefacts back, normalizing the `// From:`
# comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

gofmt -w "$GENERATED_DIR"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
