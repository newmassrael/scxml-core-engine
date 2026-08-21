#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/send_namelist_over_http/
# send_namelist_over_http_sm.go from the canonical fixture at
# integration_resources/send_namelist_over_http/send_namelist_over_http.scxml.
#
# Mirrors scripts/regen_send_namelist_over_http.sh (Rust). Only `*_sm.go`
# is copied back, so a hand-authored `*_test.go` next to the generated
# files is never touched.
#
# Usage (from repo root):
#   scripts/regen_send_namelist_over_http_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#   RUNNING the generated test additionally needs the W3C harness server
#   (`node tests/w3c/standalone_http_server.js 8080 /test`), because the
#   fixture posts over BasicHTTP.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/send_namelist_over_http/send_namelist_over_http.scxml"
GENERATED_DIR="backends/go/tests/integration/send_namelist_over_http"
STEM="send_namelist_over_http"
INPUT_ROOT="integration_resources/send_namelist_over_http"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. `--input-root` overrides the default
# §synth-6.2.6 source-hash root so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate, for a fixture that grows an inline
# `<invoke><content>` later. `--parent-stem` rewrites each child's
# `package <child>` header to the parent's.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale `*_sm.go` so a renamed synth-invoke does not leave
# the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Step 6: copy the artefacts back and normalize the embedded `// From:`
# comment to the canonical fixture path.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
