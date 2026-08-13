#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/parallel_regions_take_own_transitions/*_sm.go
# from the canonical fixture at
# integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml.
#
# Mirrors scripts/regen_parallel_regions_take_own_transitions.sh (Rust). The
# TMP-staging pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke
# children out of the canonical fixture root during the codegen run. Only
# `*_sm.go` is copied back, so any hand-authored `*_test.go` next to the
# generated files is never touched.
#
# The generated tree lives under `backends/go/tests/integration/<stem>/`
# rather than `backends/go/tests/<stem>/` so the W3C IRP and integration
# trees stay disjoint at the directory level.
#
# Usage (from repo root):
#   scripts/regen_parallel_regions_take_own_transitions_go.sh
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

FIXTURE="integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml"
GENERATED_DIR="backends/go/tests/integration/parallel_regions_take_own_transitions"
STEM="parallel_regions_take_own_transitions"
INPUT_ROOT="integration_resources/parallel_regions_take_own_transitions"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. `--input-root` overrides the default
# drift-header source-hash root so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# `package <child>` header to the parent's package `<STEM>`.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale `*_sm.go` files so a renamed synth-invoke does not
# leave the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Step 6: copy only the `*_sm.go` artefacts back, normalizing the
# `// From:` comment to the canonical fixture directory.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
