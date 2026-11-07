#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Kotlin machine under
# backends/kotlin/method-size/src/main/kotlin/com/sce/methodsize/jvm_method_size_static/
# from the document of backends/kotlin/method-size/fixtures/.
#
# The document's transition content, written into one method as the Kotlin
# machine once wrote it, passes the 65,535 bytes of code one JVM method holds,
# and it has more states than one dispatch method holds arms (gen_document.py
# beside it says why and writes it). It is not under `integration_resources/`:
# a stem there is a contract with every channel, and the question this answers
# belongs to the JVM's class file, which only the Kotlin channel compiles to.
#
# It is not under `backends/kotlin/tests/` either: the module of its own it
# lives in compiles it once, rather than in every compile of `:sce-kotlin-tests`
# (backends/kotlin/method-size/build.gradle.kts says why). Its document carries
# no script-engine language, so it has no other-language tree and takes no
# `SCE_KOTLIN_GENERATED_ROOT`.
#
# Usage (from repo root):
#   scripts/regen_jvm_method_size_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

INPUT_ROOT="backends/kotlin/method-size/fixtures"
GENERATED_ROOT="backends/kotlin/method-size/src/main/kotlin/com/sce/methodsize"
PACKAGE_PREFIX="com.sce.methodsize"
STEM="jvm_method_size_static"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"

FIXTURE="$INPUT_ROOT/$STEM.scxml"
GENERATED_DIR="$GENERATED_ROOT/$STEM"

# Step 2: stage the fixture into a tmp dir so anything the generator writes
# beside its input lands outside the canonical fixture root.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. `--input-root` overrides the default drift-header
# source-hash root; `--kotlin-package-prefix` flips the emitted `package`
# header so the driver beside it can import the machine.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Step 4: clear stale Sm.kt files so a renamed artefact does not leave the
# previous one orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
sce_generated_tree_clear "$GENERATED_DIR"

# Step 5: copy the Kotlin artefacts back, normalizing the `// Source:` comment
# to the canonical fixture directory.
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
