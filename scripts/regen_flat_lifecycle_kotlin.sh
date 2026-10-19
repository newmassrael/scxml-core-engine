#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/flat_lifecycle/
# from the canonical fixture at
# integration_resources/flat_lifecycle/flat_lifecycle.scxml.
#
# Mirrors scripts/regen_flat_lifecycle.sh (Rust). The document declares its
# transition function (`sce:transition-function="emit"`,
# docs/SCE_TRANSITION_FUNCTION.md), so generation writes the machine,
# `flat_lifecycleSm.kt`, and the function, `flat_lifecycleTransitionFunction.kt`,
# whose package is `com.sce.integration.flat_lifecycle_transition_function`. Both
# files live in this one directory: Kotlin does not tie a package to a directory.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots
# stay disjoint. `--kotlin-package-prefix com.sce.integration` flips
# the `package` header of both files to match.
#
# Usage (from repo root):
#   scripts/regen_flat_lifecycle_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/flat_lifecycle/flat_lifecycle.scxml"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/flat_lifecycle"
STEM="flat_lifecycle"
INPUT_ROOT="integration_resources/flat_lifecycle"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# Step 2: stage the fixture into a tmp dir, so anything the generator writes
# beside its input lands outside the tracked tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. `--input-root` overrides the default drift-header
# source-hash root so the embedded hash reflects the tracked fixture location
# instead of the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: clear the generated files of the last run, so a renamed artefact does
# not leave the previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 \( -name '*Sm.kt' -o -name '*TransitionFunction.kt' \) -delete

# Step 5: copy the Kotlin artefacts back, normalizing the `// Source:` comment
# to the canonical fixture directory.
for src in "$TMP"/*Sm.kt "$TMP"/*TransitionFunction.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$TMP"/*TransitionFunction.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
