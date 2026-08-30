#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/
# send_namelist_over_http/ from the canonical fixture at
# integration_resources/send_namelist_over_http/send_namelist_over_http.scxml.
#
# Mirrors scripts/regen_send_namelist_over_http.sh (Rust), with the
# Kotlin package-prefix flip the other integration stems use.
#
# Usage (from repo root):
#   scripts/regen_send_namelist_over_http_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#   The Kotlin channel has no BasicHTTP test harness yet, so this tree is
#   generated and compiled but not driven — the fixture posts over HTTP.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/send_namelist_over_http/send_namelist_over_http.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/send_namelist_over_http"
STEM="send_namelist_over_http"
INPUT_ROOT="integration_resources/send_namelist_over_http"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree during this run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. `--kotlin-package-prefix` flips the emitted
# `package` header from `com.sce.generated.<stem>` to `<prefix>.<stem>`.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: per-child generate, for a fixture that grows an inline
# `<invoke><content>` later.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
done

# Step 5: clear stale Sm.kt so a renamed synth-invoke does not leave the
# previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Step 6: copy the Kotlin artefacts back and normalize the `// Source:`
# comment to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
