#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/an_event_no_processor_delivered_has_no_origin/*_sm.go
# from the canonical fixture at
# integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml.
#
# Mirrors the Rust generation of the same fixture
# (tools/codegen/trees/rust-integration.toml), with the same
# `--host-processor x-sce-host` declaration the driver registers a processor
# for. Only `*_sm.go` is copied back, so the hand-authored `*_test.go` next to
# the generated file is never touched.
#
# Usage (from repo root):
#   scripts/regen_an_event_no_processor_delivered_has_no_origin_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml"
GENERATED_DIR="backends/go/tests/integration/an_event_no_processor_delivered_has_no_origin"
STEM="an_event_no_processor_delivered_has_no_origin"
INPUT_ROOT="integration_resources/an_event_no_processor_delivered_has_no_origin"
HOST_PROCESSOR="x-sce-host"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# `--input-root` keeps the drift-header source hash on the tracked fixture
# location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the `// From:` comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
