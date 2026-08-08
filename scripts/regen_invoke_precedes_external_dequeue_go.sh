#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/invoke_precedes_external_dequeue/{invoke_precedes_external_dequeue,
# invoke_precedes_external_dequeue__sce_synth_invoke__inv_*}_sm.go from the
# canonical fixture at
# integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml.
#
# Mirrors scripts/regen_invoke_precedes_external_dequeue.sh (Rust). The TMP-staging
# pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke children
# (sce-build/src/parser.rs:1804-1805) out of the canonical fixture root
# during the codegen run. Only `*_sm.go` is copied back, so the
# hand-authored `invoke_precedes_external_dequeue_test.go` next to the generated
# files is never touched.
#
# The generated tree lives under `backends/go/tests/integration/<stem>/`
# rather than `backends/go/tests/<stem>/` so the W3C IRP and integration
# trees stay disjoint at the directory level.
#
# Usage (from repo root):
#   scripts/regen_invoke_precedes_external_dequeue_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line that the codegen emits
# on every invocation. `source-hash` and `template-hash` stay
# deterministic for unchanged fixture + template inputs.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml"
GENERATED_DIR="backends/go/tests/integration/invoke_precedes_external_dequeue"
STEM="invoke_precedes_external_dequeue"
INPUT_ROOT="integration_resources/invoke_precedes_external_dequeue"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits parent <stem>_sm.go into $TMP AND writes
# the split-out synth-invoke children (Mesh §9.6.6 rule 1) into $TMP
# next to the staged parent.
#
# `--input-root` overrides the default §synth-6.2.6 source-hash root (the
# SCXML file's parent) so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# `package <child>` header to the parent's package `<STEM>` so the
# parent's unqualified references to the child's exported types
# resolve in the shared package.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale `*_sm.go` files in the tracked generated tree so
# a renamed synth-invoke (e.g. when the fixture's invoke `id` changes)
# doesn't leave the previous artefact orphaned next to the new one.
# The hand-authored `*_test.go` next to the generated tree is excluded
# from this glob and stays untouched.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Step 6: copy only the `*_sm.go` artefacts back into the tracked
# generated tree. Normalize the `// From:` comment so the tracked
# output points at the canonical fixture directory instead of the
# transient $TMP path — mirrors the `--input-root` override at hash
# time.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
