#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/event_origin_is_a_location/
# from the canonical fixture at
# integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml.
#
# Mirrors scripts/regen_event_origin_is_a_location.sh (Rust). The TMP-staging
# pattern keeps SCE Mesh §9.6.6 rule 1's adjacent synth-invoke children
# (sce-build/src/parser.rs:1804-1805) out of the canonical fixture root
# during the codegen run. integration_resources/event_origin_is_a_location/
# stays a parent-only canonical surface; synth-invoke children are
# derived and live only in $TMP.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots
# stay disjoint. `--kotlin-package-prefix com.sce.integration` flips
# the `package` header on every emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_event_origin_is_a_location_kotlin.sh
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

FIXTURE="integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml"
GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/event_origin_is_a_location"
STEM="event_origin_is_a_location"
INPUT_ROOT="integration_resources/event_origin_is_a_location"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree during this run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits parent <Stem>Sm.kt into $TMP AND writes
# the split-out synth-invoke children (Mesh §9.6.6 rule 1) into $TMP
# next to the staged parent.
#
# `--input-root` overrides the default §synth-6.2.6 source-hash root (the
# SCXML file's parent) so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path.
# `--kotlin-package-prefix` flips the emitted `package` header from
# the default `com.sce.generated.<stem>` to `<prefix>.<stem>`.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: per-child generate. `--parent-stem` rewrites each child's
# `package <prefix>.<child>` header to the parent's package
# `<prefix>.<STEM>` so the parent's unqualified references to the
# child StateMachine class resolve in the shared package.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
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
