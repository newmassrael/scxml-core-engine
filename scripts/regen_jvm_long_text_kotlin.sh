#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Kotlin machines under
# backends/kotlin/tests/src/main/kotlin/com/sce/integration/jvm_long_text_static/ and
# backends/kotlin/tests/src/main/kotlin/com/sce/integration/jvm_long_text_ecmascript/
# from the two documents of backends/kotlin/tests/fixtures/jvm_long_text/.
#
# The documents carry texts past the 65,535 bytes of modified UTF-8 one JVM
# class-file constant holds, at every place the Kotlin backend writes document
# text as an expression (gen_documents.py beside them says why and writes them).
# They are not under `integration_resources/`: a stem there is a contract with
# every channel, and the question these answer belongs to the JVM's class file,
# which only the Kotlin channel compiles to.
#
# Usage (from repo root):
#   scripts/regen_jvm_long_text_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable for unchanged inputs and generator.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

INPUT_ROOT="backends/kotlin/tests/fixtures/jvm_long_text"
GENERATED_ROOT="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration"
PACKAGE_PREFIX="com.sce.integration"

# Step 1: resolve sce-codegen, building it when no profile holds one.
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"

for STEM in jvm_long_text_static jvm_long_text_ecmascript; do
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

    rm -rf "$TMP"
    trap - EXIT
    echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
done
