#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-rust-tests/src/integration/donedata_local_invoke/ from the
# hand-authored fixture at sce-rust-tests/fixtures/donedata_local_invoke.scxml.
#
# Why a script (vs the per-test ad-hoc workflow): SCE Mesh §9.6.6 rule 1
# pins synth-invoke child SCXMLs to the same directory as their parent
# (sce-build/src/parser.rs:1804-1805 — `scxml_dir.join(...)`). Running
# `sce-codegen generate sce-rust-tests/fixtures/donedata_local_invoke.scxml`
# therefore writes two transient children adjacent to the fixture itself —
# polluting the tracked `sce-rust-tests/fixtures/` directory with codegen
# side effects on every regen.
#
# This script wraps the documented per-test workflow (`tests/donedata_local_invoke.rs`
# header comment) so regen-ability isn't lost: copy the fixture into a tmp
# directory, run the parent generate + per-child --as-child passes there,
# copy only the `*.rs` artefacts back into the tracked integration/ tree,
# then discard the tmp.
#
# Usage (from repo root):
#   scripts/regen_donedata_local_invoke.sh
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
FIXTURE="sce-rust-tests/fixtures/donedata_local_invoke.scxml"
GENERATED_DIR="sce-rust-tests/src/integration/donedata_local_invoke"
STEM="donedata_local_invoke"

# Step 1: build sce-codegen in release mode if absent.
if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

# Step 2: stage the fixture into a tmp dir so synth-invoke children land
# outside the tracked fixtures/ tree.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

# Step 3: parent generate. Emits parent _sm.rs into $TMP AND writes the
# split-out synth-invoke children (Mesh §9.6.6 rule 1) into $TMP next
# to the staged parent.
#
# `--input-root` overrides the default §6.2.6 source-hash root (the
# SCXML file's parent) so the embedded hash reflects the tracked
# fixture location instead of the transient $TMP path — a stranger
# running `sce-codegen verify sce-rust-tests/src/integration/donedata_local_invoke
# --input-root sce-rust-tests/fixtures` then reproduces the same hash.
INPUT_ROOT="sce-rust-tests/fixtures"
"$CODEGEN" generate "$TMP/$STEM.scxml" -l rust -o "$TMP/" \
    --input-root "$INPUT_ROOT"

# Step 4: per-child generate. Each synth-invoke child becomes its own
# state machine via --as-child --parent-stem so the emitted module names
# (and the package layout for Kotlin/Go consumers) match the parent's
# crate context. The same `--input-root` override keeps every emitted
# header pointing at the tracked fixture directory.
for child in "$TMP"/"${STEM}"__sce_synth_invoke__*.scxml; do
    [[ -f "$child" ]] || continue
    "$CODEGEN" generate "$child" \
        --as-child --parent-stem "$STEM" \
        -l rust -o "$TMP/" \
        --input-root "$INPUT_ROOT"
done

# Step 5: clear stale _sm.rs files in the tracked generated tree so a
# renamed synth-invoke (e.g. when the fixture's invoke `id` changes)
# doesn't leave the previous artefact orphaned next to the new one.
# The directory always re-materialises from $TMP below.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete

# Step 6: copy only the Rust artefacts back into the tracked integration/
# tree. The SCXML side files in $TMP are intentionally left behind; this
# script's contract is that fixtures/ stays a one-file hand-authored
# surface (parent only) and integration/ stays the codegen output tree
# (Rust files only).
#
# The Rust backend's template embeds the absolute path of the input
# SCXML in a `// From:` comment block. Because we staged into $TMP,
# that path would otherwise be nondeterministic (a different
# `/tmp/tmp.XXXXXX` per regen). Rewrite the comment in place so the
# tracked output points at the canonical input location, mirroring
# the `--input-root` override used at hash time.
for src in "$TMP"/*.rs; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Step 7: regenerate mod.rs. sce-codegen does not emit it for the
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

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
