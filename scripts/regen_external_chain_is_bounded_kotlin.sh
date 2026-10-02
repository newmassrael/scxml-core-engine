#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Kotlin machine under
# backends/kotlin/tests/src/main/kotlin/com/sce/integration/external_chain_is_bounded/
# from tests/integration/external_chain_is_bounded.scxml.
#
# Mirrors scripts/regen_external_chain_is_bounded.sh (Rust), which says why the
# document is not under `integration_resources/`. `--kotlin-package-prefix`
# flips the emitted `package` header off `com.sce.generated.<stem>` so the driver
# beside it can import the machine.
#
# Usage (from repo root):
#   scripts/regen_external_chain_is_bounded_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="tests/integration/external_chain_is_bounded.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/external_chain_is_bounded"
STEM="external_chain_is_bounded"
INPUT_ROOT="tests/integration"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so anything the generator writes
# beside its input lands outside the canonical fixture root.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. `--input-root` overrides the default drift-header
# source-hash root; `--kotlin-package-prefix` flips the emitted `package` header.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: clear stale Sm.kt files so a renamed artefact does not leave the
# previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Step 5: copy the Kotlin artefacts back, normalizing the `// Source:` comment
# to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
