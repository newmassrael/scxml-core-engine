#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/ai_loop/ from the worked example at
# examples/ai_loop/ai_loop.scxml.
#
# The Go half of what `scripts/regen_ai_loop.sh` does for Rust, and it exists
# for the reason that script's header names: a clause asserted in one channel
# is that engine's word for the document rather than the document's own. Two
# channels made the claim "two engines, one document"; a third makes it a claim
# about the document. `sce-build/tests/ai_loop_channel_parity.rs` is what holds
# every registered channel to the same scenario set, and this script is how the
# Go one gets a machine to assert against.
#
# The input is an EXAMPLE, not a fixture under `integration_resources/`, so the
# `generate-integration -l go` fan-out in `regen_all_committed_trees.sh` does
# not reach it and this script is named there explicitly.
#
# Usage (from repo root):
#   scripts/regen_ai_loop_go.sh
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
GENERATED_DIR="backends/go/tests/integration/ai_loop"
STEM="ai_loop"
INPUT_ROOT="examples/ai_loop"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Staged into a tmp dir so the split-out synth-invoke children SCE Mesh §9.6.6
# rule 1 writes next to the parent land outside the tracked `examples/` tree.
cp "$FIXTURE" "$TMP/$STEM.scxml"

# W3C SCXML 6.2.5: the document declares its acts as sends the HOST serves. The
# same declaration is spelled in `scripts/regen_ai_loop.sh`,
# `examples/ai_loop/CMakeLists.txt` and `tests/CMakeLists.txt`; without it here
# codegen emits the refusal and every act in the Go channel raises
# `error.execution` instead of reaching a host.
#
# `--input-root` overrides the source-hash root so the embedded hash reflects
# the tracked example directory rather than the transient $TMP path.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l go -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --host-processor x-sce-host

# The document carries no `<invoke>` today, so this loop runs zero times. It is
# here rather than left out because `scripts/regen_ai_loop.sh` handles the same
# case on the Rust side: an example that grows an inline `<invoke><content>`
# child would otherwise leave this channel generating a parent that references
# types no file in the package defines, and the two channels would disagree
# about the document at the next edit rather than at this one.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l go -o "$TMP/" \
        --input-root "$INPUT_ROOT" \
        --host-processor x-sce-host
done

# Clear stale `*_sm.go` first so a renamed synth-invoke does not leave the
# previous artefact orphaned beside the new one. The hand-authored
# `ai_loop_test.go` next to the generated files is outside this glob.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete

# Normalise the `// From:` comment so the committed output points at the
# tracked example rather than the transient $TMP path — the comment half of
# what `--input-root` did for the hash.
for src in "$TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
