#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/a_host_names_its_origin_in_its_own_terms/
# from the canonical fixture at
# integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml.
#
# docs/adr/0090, test 3. The document has its host run `<invoke type="x-sce-host">`,
# so it is generated with `--host-invoker x-sce-host`, the type its driver
# registers (as `sce_generate_static_integration_test(... HOST_INVOKER x-sce-host)`
# builds the C++ machine): generated without it, the invoke is refused at build
# time rather than dispatched to the host.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots
# stay disjoint. `--kotlin-package-prefix com.sce.integration` flips
# the `package` header on every emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_a_host_names_its_origin_in_its_own_terms_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/a_host_names_its_origin_in_its_own_terms"
STEM="a_host_names_its_origin_in_its_own_terms"
INPUT_ROOT="integration_resources/a_host_names_its_origin_in_its_own_terms"
PACKAGE_PREFIX="com.sce.integration"

# Kept in one place here and named again by the driver
# (AHostNamesItsOriginInItsOwnTermsTest), so the type the build was told and the
# type the host registers are the same string.
HOST_INVOKER="x-sce-host"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX" \
    --host-invoker "$HOST_INVOKER"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the `// Source:` comment to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-invoker $HOST_INVOKER)"
