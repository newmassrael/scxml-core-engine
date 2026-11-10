#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/statechart_native_action/ from the
# canonical W3C SCXML G.7 `<sce:action>` fixture at
# sce-build/tests/fixtures/event_schema/statechart_native_action.scxml.
#
# The Go tree of the fixture whose Rust tree
# `tools/codegen/trees/rust-integration.toml` declares. Named for the
# document's own stem rather than for the axis, unlike the Rust tree: a Go
# package name comes from the emitted file, so a directory named anything else
# puts two package names in one directory and the module stops building.
#
# The `// From:` rewrite is the same one every `regen_*_go.sh` does — the
# tracked artefact has to point at the canonical fixture rather than at the
# temporary directory this run happened to use, or the tree stops reproducing
# on another machine.
#
# Usage (from repo root):
#   scripts/regen_native_action_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_native_action.scxml"
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="backends/go/tests/integration/statechart_native_action"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$TMP/" --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
