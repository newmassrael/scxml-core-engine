#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/parallel_self_transition_keeps_its_leaf/
# from the canonical fixture at
# integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml.
#
# Mirrors scripts/regen_parallel_self_transition_keeps_its_leaf.sh (Rust). The
# TMP-staging pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke
# children out of the canonical fixture root during the codegen run.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots stay
# disjoint. `--kotlin-package-prefix com.sce.integration` flips the
# `package` header on every emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_parallel_self_transition_keeps_its_leaf_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line that the codegen emits
# on every invocation.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml"
GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/parallel_self_transition_keeps_its_leaf"
STEM="parallel_self_transition_keeps_its_leaf"
INPUT_ROOT="integration_resources/parallel_self_transition_keeps_its_leaf"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the canonical fixture root during this run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. `--input-root` overrides the default
# drift-header source-hash root; `--kotlin-package-prefix` flips the
# emitted `package` header from `com.sce.generated.<stem>`.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# package header to the parent's so unqualified references resolve.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
done

# Step 5: clear stale Sm.kt files so a renamed synth-invoke does not
# leave the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Step 6: copy the Kotlin artefacts back, normalizing the `// Source:`
# comment to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
