#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/a_child_timer_is_a_deadline_of_its_parent/
# from tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml.
#
# The source is NOT under `integration_resources/`, and that is deliberate.
# A stem there is a seven-channel contract — C++ Interpreter, C++ AOT, Rust,
# Go, Python, Kotlin, C11 — that `integration_stem_registration.rs` enforces,
# and the question this document asks is a host's: "when does this machine next
# need a tick?". C11 and the C++ Interpreter expose no such query, so there is
# nothing for their drivers to assert, and an exemption in that table is for a
# missing HARNESS, never for a missing feature. The document lives beside its
# drivers on the same footing as `parallel_region_root_external_domain`.
#
# Usage (from repo root):
#   scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/a_child_timer_is_a_deadline_of_its_parent"
STEM="a_child_timer_is_a_deadline_of_its_parent"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

# The inline child is emitted beside its parent (`<stem>__sce_synth_invoke__<id>_sm.rs`),
# so the module list is rebuilt from what the run produced.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
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
