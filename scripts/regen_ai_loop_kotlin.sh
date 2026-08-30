#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/ai_loop/
# from the worked example at examples/ai_loop/ai_loop.scxml.
#
# The Kotlin half of what `scripts/regen_ai_loop.sh` (Rust) and
# `scripts/regen_ai_loop_go.sh` (Go) do, and it exists for the reason those
# headers name: a clause asserted in one channel is that engine's word for the
# document rather than the document's own.
# `sce-build/tests/ai_loop_channel_parity.rs` holds every registered channel to
# the same scenario set, and this script is how the Kotlin one gets a machine to
# assert against.
#
# A per-stem script rather than the `generate-w3c` fan-out the Gradle build
# runs, for the reason `regen_host_processor_kotlin.sh` gives: that fan-out
# enumerates the W3C corpus and has no per-stem flags, and this input is an
# EXAMPLE rather than a stem under `integration_resources/`.
#
# Usage (from repo root):
#   scripts/regen_ai_loop_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable. `scripts/lib/sce_codegen.sh` pins
# `generated-at` to SOURCE_DATE_EPOCH=0, which is what
# `committed_trees_carry_a_pinned_generated_at` requires of a committed tree.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

FIXTURE="examples/ai_loop/ai_loop.scxml"
INPUT_ROOT="examples/ai_loop"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/ai_loop"
PACKAGE_PREFIX="com.sce.integration"

# Kept in one place here and asserted from the driver, so the string the build
# was given and the string the host registers cannot drift into two. W3C SCXML
# 6.2.5: the document declares its acts as sends the HOST serves, and the same
# declaration is spelled in `scripts/regen_ai_loop.sh`,
# `scripts/regen_ai_loop_go.sh`, `examples/ai_loop/CMakeLists.txt` and
# `tests/CMakeLists.txt`. Without it codegen emits the refusal and every act in
# this channel raises `error.execution` instead of reaching a host.
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Staged into a tmp dir so the split-out synth-invoke children SCE Mesh §9.6.6
# rule 1 writes next to the parent land outside the tracked `examples/` tree.
# `--input-root` then points the embedded source-hash at the tracked example
# directory rather than the transient path.
cp "$FIXTURE" "$TMP/ai_loop.scxml"

"$CODEGEN" generate "$TMP/ai_loop.scxml" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX" \
    --host-processor "$HOST_PROCESSOR"

# The document carries no `<invoke>` today, so this loop runs zero times. It is
# here rather than left out because both sibling scripts handle the same case:
# an example that grows an inline `<invoke><content>` child would otherwise
# leave this channel generating a parent that references types no file in the
# package defines, and the channels would disagree about the document at the
# next edit rather than at this one.
for child in "$TMP"/ai_loop__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem ai_loop \
        -l kotlin -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX" \
        --host-processor "$HOST_PROCESSOR"
done

# Clear stale `*Sm.kt` first so a renamed synth-invoke does not leave the
# previous artefact orphaned beside the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
