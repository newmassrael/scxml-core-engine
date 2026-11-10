#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/an_origin_is_the_address_its_sender_published/*_sm.go
# from the canonical fixture at
# integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml.
#
# Mirrors the Rust generation of the same fixture
# (tools/codegen/trees/rust-integration.toml): the fixture is staged into a tmp
# dir so the split-out synth-invoke child lands outside the tracked tree, each
# child is generated into the parent's package, and only `*_sm.go` is copied
# back, so the hand-authored `*_test.go` beside them is never touched.
#
# Usage (from repo root):
#   scripts/regen_an_origin_is_the_address_its_sender_published_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml"
GENERATED_DIR="backends/go/tests/integration/an_origin_is_the_address_its_sender_published"
STEM="an_origin_is_the_address_its_sender_published"
INPUT_ROOT="integration_resources/an_origin_is_the_address_its_sender_published"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# `--input-root` keeps the drift-header source hash on the tracked fixture
# location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# `--parent-stem` puts each child in the parent's package, so the parent's
# unqualified references to the child's exported types resolve.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the `// From:` comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
