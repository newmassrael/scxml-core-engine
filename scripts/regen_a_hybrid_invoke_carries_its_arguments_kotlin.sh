#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Kotlin integration tree for
# a_hybrid_invoke_carries_its_arguments from the canonical fixture at
# integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml.
#
# Mirrors scripts/regen_a_hybrid_invoke_carries_its_arguments.sh (Rust).
# `--parent-stem` puts each candidate in the parent's package: Kotlin gives
# every document a package of its own, so without it the parent's reference
# to a candidate class does not resolve.
#
# Usage (from repo root):
#   scripts/regen_a_hybrid_invoke_carries_its_arguments_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

STEM="a_hybrid_invoke_carries_its_arguments"
FIXTURE_DIR="integration_resources/$STEM"
FIXTURE="$FIXTURE_DIR/$STEM.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/$STEM"
PACKAGE_PREFIX="com.sce.integration"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"
# ⚠ Generated from the TRACKED document, not from the copy the parent run
# staged beside itself, so the `// From:` line of a committed artefact names
# a path that is the same on every run.
for child in "$TMP"/*.scxml; do
    [ -e "$child" ] || continue
    base="$(basename "$child")"
    [ "$base" = "$STEM.scxml" ] && continue
    src="$FIXTURE_DIR/$base"
    [ -f "$src" ] || src="$child"
    "$CODEGEN" generate "$src" --as-child --parent-stem "$STEM" -l kotlin -o "$TMP/" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
