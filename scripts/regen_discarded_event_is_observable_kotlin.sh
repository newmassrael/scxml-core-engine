#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate
# backends/kotlin/tests/src/main/kotlin/com/sce/integration/discarded_event_is_observable/
# from the canonical fixture at
# integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml.
#
# Mirrors scripts/regen_discarded_event_is_observable.sh (Rust). Only `*Sm.kt`
# is copied back, so the hand-authored driver under `src/test/kotlin/` is never
# touched.
#
# Usage (from repo root):
#   scripts/regen_discarded_event_is_observable_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml"
GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/discarded_event_is_observable"
STEM="discarded_event_is_observable"
INPUT_ROOT="integration_resources/discarded_event_is_observable"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree during this run.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. `--kotlin-package-prefix` flips the emitted
# `package` header from the default `com.sce.generated.<stem>` to
# `<prefix>.<stem>`.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: per-child generate for any synth-invoke child.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
done

# Step 5: clear stale Sm.kt files so a renamed synth-invoke does not leave the
# previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

# Step 6: copy the Kotlin artefacts back, normalizing the `// Source:` comment
# onto the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
