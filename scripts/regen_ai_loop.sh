#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/ai_loop/ from the worked
# example at examples/ai_loop/ai_loop.scxml.
#
# The input is an EXAMPLE, not a fixture under `integration_resources/`.
# That is deliberate and it is what keeps the two channels honest about the
# same document: `examples/ai_loop/ai_loop_example.cpp` drives it through the
# C++ AOT engine and `backends/rust/tests/tests/ai_loop.rs` drives it through
# the Rust one, so a topology change that only one engine honours fails on
# the other. A document that ships as a worked example is worth more than
# one engine's word for it.
#
# `integration_stem_registration` enumerates `integration_resources/` and so
# does not reach this stem: the example is not on the seven-channel contract,
# and promoting it there would demand five more drivers for a document whose
# job is to be read. Two engines is the claim being made here, and both are
# asserted.
#
# Usage (from repo root):
#   scripts/regen_ai_loop.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="examples/ai_loop/ai_loop.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/ai_loop"
STEM="ai_loop"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# §scxml-6.2.5: the document declares its acts as sends the HOST serves. The
# same declaration is spelled in `examples/ai_loop/CMakeLists.txt` and
# `tests/CMakeLists.txt`; without it here codegen emits the refusal and every
# act in the Rust channel raises `error.execution` instead of reaching a host.
"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-processor x-sce-host

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Rebuild mod.rs from the actual `_sm.rs` filenames the regen produced, so a
# document that grows an inline `<invoke><content>` child stitches itself in
# rather than failing to compile against a hand-maintained list.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_ai_loop.sh)"
    echo ""
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

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
