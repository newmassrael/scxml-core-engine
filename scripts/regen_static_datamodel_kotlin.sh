#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/static_counter/
# from sce-build/tests/fixtures/static_datamodel/static_counter.scxml.
#
# The Kotlin compile+run gate for datamodel="sce-static"
# (docs/SCE_ACCEPTED_SUBSET.md §2.15). The committed machine compiles as part
# of `:sce-kotlin-tests`, so its variables really are Kotlin fields and every
# lowered expression really type-checks; StaticDatamodelTest.kt drives it with
# no script engine. Driven by its own script rather than the
# `generate-integration` fan-out because only Kotlin lowers the model today —
# the other backends refuse the document (`STATIC_DATAMODEL_BACKENDS`).
#
# Usage (from repo root):
#   scripts/regen_static_datamodel_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/static_datamodel/static_counter.scxml"
INPUT_ROOT="sce-build/tests/fixtures/static_datamodel"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/static_counter"
PACKAGE_PREFIX="com.sce.integration"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
