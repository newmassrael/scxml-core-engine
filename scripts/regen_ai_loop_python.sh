#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/ai_loop/ from the worked example
# at examples/ai_loop/ai_loop.scxml.
#
# The Python half of what `scripts/regen_ai_loop.sh` (Rust),
# `scripts/regen_ai_loop_go.sh` and `scripts/regen_ai_loop_kotlin.sh` do, and it
# exists for the reason those headers name: a clause asserted in one channel is
# that engine's word for the document rather than the document's own.
# `sce-build/tests/ai_loop_channel_parity.rs` holds every registered channel to
# the same scenario set, and this script is how the Python one gets a machine to
# assert against.
#
# A per-stem script rather than the `generate-integration` fan-out, because the
# input is an EXAMPLE rather than a stem under `integration_resources/` and that
# fan-out enumerates the stems.
#
# Usage (from repo root):
#   scripts/regen_ai_loop_python.sh
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
GENERATED_DIR="backends/python/tests/integration/ai_loop"
STEM="ai_loop"

# Kept in one place here and asserted from the driver, so the string the build
# was given and the string the host registers cannot drift into two. W3C SCXML
# 6.2.5: the document declares its acts as sends the HOST serves, and the same
# declaration is spelled in the three sibling regen scripts,
# `examples/ai_loop/CMakeLists.txt` and `tests/CMakeLists.txt`. Without it
# codegen emits the refusal and every act in this channel raises
# `error.execution` instead of reaching a host.
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Staged into a tmp dir so the split-out synth-invoke children SCE Mesh §9.6.6
# rule 1 writes next to the parent land outside the tracked `examples/` tree.
# `--input-root` then points the embedded source-hash at the tracked example
# directory rather than the transient path.
cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --host-processor "$HOST_PROCESSOR"

# The document carries no `<invoke>` today, so this loop runs zero times. It is
# here rather than left out because all three sibling scripts handle the same
# case: an example that grows an inline `<invoke><content>` child would
# otherwise leave this channel generating a parent that references names no
# module in the package defines, and the channels would disagree about the
# document at the next edit rather than at this one.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l python -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --host-processor "$HOST_PROCESSOR"
done

# Clear stale `*_sm.py` first so a renamed synth-invoke does not leave the
# previous artefact orphaned beside the new one. The hand-authored
# `test_ai_loop_aot.py` next to the generated files is outside this glob.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

# Normalise the `# From:` comment so the committed output points at the tracked
# example rather than the transient $TMP path — the comment half of what
# `--input-root` did for the hash.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
