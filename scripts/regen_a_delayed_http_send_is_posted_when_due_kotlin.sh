#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/a_delayed_http_send_is_posted_when_due/
# from the canonical fixture at
# integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml.
#
# Mirrors scripts/regen_a_delayed_http_send_is_posted_when_due.sh (Rust). The tree
# lives under `com/sce/integration/`, which `--kotlin-package-prefix` names in
# every emitted `package` header.
#
# Usage (from repo root):
#   scripts/regen_a_delayed_http_send_is_posted_when_due_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/a_delayed_http_send_is_posted_when_due"
STEM="a_delayed_http_send_is_posted_when_due"
INPUT_ROOT="integration_resources/a_delayed_http_send_is_posted_when_due"
PACKAGE_PREFIX="com.sce.integration"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Normalize the `// Source:` comment to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
