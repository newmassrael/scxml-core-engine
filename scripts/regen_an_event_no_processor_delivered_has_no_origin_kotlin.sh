#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/an_event_no_processor_delivered_has_no_origin/
# from the canonical fixture at
# integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml.
#
# docs/adr/0090, test 2. The document sends `host.ask` to the host's processor
# `x-sce-host`, so it is generated with `--host-processor x-sce-host`, the type
# its driver registers (as `sce_generate_static_integration_test(... HOST_PROCESSOR
# x-sce-host)` builds the C++ machine): generated without it, the send compiles to
# the W3C SCXML 6.2 error.execution instead of a dispatch to the host.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots
# stay disjoint. `--kotlin-package-prefix com.sce.integration` flips
# the `package` header on every emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_an_event_no_processor_delivered_has_no_origin_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/an_event_no_processor_delivered_has_no_origin"
STEM="an_event_no_processor_delivered_has_no_origin"
INPUT_ROOT="integration_resources/an_event_no_processor_delivered_has_no_origin"
PACKAGE_PREFIX="com.sce.integration"

# Kept in one place here and named again by the driver
# (AnEventNoProcessorDeliveredHasNoOriginTest), so the type the build was told
# and the type the host registers are the same string.
HOST_PROCESSOR="x-sce-host"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX" \
    --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the `// Source:` comment to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
