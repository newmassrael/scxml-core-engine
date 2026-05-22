#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-kotlin-tests/src/main/kotlin/com/sce/generated/donedata_local_invoke/
# from the hand-authored fixture at
# sce-kotlin-tests/src/test/resources/fixtures/donedata_local_invoke.scxml.
#
# Mirrors scripts/regen_donedata_local_invoke.sh (Rust). The TMP-staging
# pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke children
# (sce-build/src/parser.rs:1804-1805) out of the tracked fixtures/ tree
# during the codegen run. Synth-invoke children that pre-exist in
# fixtures/ remain committed and are untouched by this script — its
# contract covers only the Kotlin output tree.
#
# Usage (from repo root):
#   scripts/regen_donedata_local_invoke_kotlin.sh
#
# Requires:
#   target/release/sce-codegen (build first if missing: see step 1).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line that the codegen emits
# on every invocation. `source-hash` and `template-hash` stay
# deterministic for unchanged fixture + template inputs.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="sce-kotlin-tests/src/test/resources/fixtures/donedata_local_invoke.scxml"
GENERATED_DIR="sce-kotlin-tests/src/main/kotlin/com/sce/generated/donedata_local_invoke"
STEM="donedata_local_invoke"
INPUT_ROOT="sce-kotlin-tests/src/test/resources/fixtures"

# Step 1: build sce-codegen in release mode if absent.
if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree during this run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits parent <Stem>Sm.kt into $TMP AND writes
# the split-out synth-invoke children (Mesh §9.6.6 rule 1) into $TMP
# next to the staged parent.
#
# `--input-root` overrides the default §6.2.6 source-hash root (the
# SCXML file's parent) so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# `package com.sce.generated.<child>` header to the parent's package
# `com.sce.generated.<STEM>` so the parent's unqualified references to
# the child StateMachine class resolve in the shared package.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale Sm.kt files in the tracked generated tree so a
# renamed synth-invoke (e.g. when the fixture's invoke `id` changes)
# doesn't leave the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Step 6: copy only the Kotlin artefacts back into the tracked
# generated tree. Normalize the `// Source:` comment so the tracked
# output points at the canonical fixture directory instead of the
# transient $TMP path — mirrors the `--input-root` override at hash
# time.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
