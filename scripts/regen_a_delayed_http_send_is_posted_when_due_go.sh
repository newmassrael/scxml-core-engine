#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/a_delayed_http_send_is_posted_when_due/*_sm.go
# from the canonical fixture at
# integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml.
#
# Mirrors scripts/regen_a_delayed_http_send_is_posted_when_due.sh (Rust). Only
# `*_sm.go` is copied back, so the hand-authored `*_test.go` next to the
# generated file is never touched.
#
# Usage (from repo root):
#   scripts/regen_a_delayed_http_send_is_posted_when_due_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml"
GENERATED_DIR="backends/go/tests/integration/a_delayed_http_send_is_posted_when_due"
STEM="a_delayed_http_send_is_posted_when_due"
INPUT_ROOT="integration_resources/a_delayed_http_send_is_posted_when_due"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# `--input-root` keeps the drift-header source hash on the tracked fixture
# location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Normalize the `// From:` comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
