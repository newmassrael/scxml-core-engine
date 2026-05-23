#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-rust-tests/src/integration/donedata_local_invoke/ from the
# canonical fixture at integration_resources/donedata_local_invoke/donedata_local_invoke.scxml.
#
# Pipeline:
#   stage parent into tmp → `sce-codegen generate` (auto-emits inline
#   children alongside the parent per SCE Mesh §9.6.6 rule 2) →
#   rewrite the embedded `// From:` path back to the canonical fixture
#   location → copy `*.rs` into the tracked tree → cargo fmt → stitch
#   mod.rs. The tmp-stage exists because `--input-root` pins the
#   `source-hash` to `integration_resources/...` while the generator
#   parses the SCXML from disk; staging keeps the parse-time path
#   stable across regens.
#
# Why the tmp-stage + `// From:` rewrite are still needed even though
# the parser is now pure (no synth-invoke disk write): the Rust
# license-header template embeds `model.scxml_source_path`, which is
# computed via `canonicalize` from the input path the generator was
# invoked with. Without the tmp-stage, that field would reflect
# whichever absolute working-tree path the caller used; with the
# tmp-stage it would reflect `/tmp/tmp.XXXXXX/...`. The sed rewrite
# pins it to the tracked canonical location so reviewers see the
# tracked path in the committed artefact.
#
# Usage (from repo root):
#   scripts/regen_donedata_local_invoke.sh
#
# Requires:
#   target/release/sce-codegen (auto-built when missing).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line that the codegen emits
# on every invocation. `source-hash` and `template-hash` stay
# deterministic for unchanged fixture + template inputs.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="integration_resources/donedata_local_invoke/donedata_local_invoke.scxml"
GENERATED_DIR="sce-rust-tests/src/integration/donedata_local_invoke"
STEM="donedata_local_invoke"
INPUT_ROOT="integration_resources/donedata_local_invoke"

# Step 1: build sce-codegen in release mode if absent.
if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

# Step 2: stage the fixture in tmp so the `// From:` path embedded in
# the license-header template (set from `canonicalize($scxml_path)`)
# is deterministic across regens.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: generate. Auto-emits the parent `_sm.rs` AND every inline
# `<invoke><content>` child's `_sm.rs` into $TMP per SCE Mesh §9.6.6
# rule 2 (codegen emit is the single materialization point for synth
# children — the parser keeps them as in-memory submodels and never
# writes a sibling `.scxml`). `--input-root` pins the §6.2.6
# `source-hash` to the tracked fixture dir so a stranger running
# `sce-codegen verify $GENERATED_DIR --input-root $INPUT_ROOT`
# reproduces the same hash from the canonical input.
"$CODEGEN" generate "$TMP/$STEM.scxml" -l rust -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: clear stale `_sm.rs` files in the tracked generated tree so
# a renamed synth-invoke (e.g. when the fixture's invoke `id` changes)
# doesn't leave the previous artefact orphaned next to the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete

# Step 5: rewrite the embedded `// From: $TMP/...` to point at the
# tracked canonical fixture path, then copy into the integration tree.
# The SCXML side file in $TMP is intentionally left behind — this
# script's contract is that `integration_resources/donedata_local_invoke/`
# stays the canonical fixture root (parent only) and
# `$GENERATED_DIR/` stays the codegen output tree (Rust files only).
for src in "$TMP"/*.rs; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Step 6: regenerate mod.rs. sce-codegen does not emit it for the
# single-file `generate` mode (each invocation only knows its own
# module), so the script reads the actual `*_sm.rs` filenames the
# regen produced and stitches them into a fresh mod.rs. Sorted output
# keeps the file diff-stable across regens.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_donedata_local_invoke.sh)"
    echo ""
    # Parent module first, then sorted children — matches the existing
    # hand-maintained ordering convention in this directory.
    parent_stem="${STEM}_sm"
    echo "mod ${parent_stem};"
    echo "pub use ${parent_stem}::*;"
    while IFS= read -r child_stem; do
        echo "mod ${child_stem};"
        echo "pub use ${child_stem}::*;"
    done < <(
        find "$GENERATED_DIR" -maxdepth 1 -name "${STEM}__sce_synth_invoke__*_sm.rs" \
            -printf '%f\n' | sed 's/\.rs$//' | sort
    )
} > "$MODRS"

# Step 7: apply `cargo fmt` to the regenerated tree. The codegen
# emits unformatted (raw template) Rust; the pre-commit hook gate
# requires fmt-clean output. Without this step every regen would
# produce a working-tree dirty state against the previously-fmt-applied
# committed baseline (the root cause of the historical
# donedata_local_invoke staleness — see commit ac701ec4a applying
# fmt + 0803c08ff bumping templates without a corresponding regen).
cargo fmt -p sce-rust-tests

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
