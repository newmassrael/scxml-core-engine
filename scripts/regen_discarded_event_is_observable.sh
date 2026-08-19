#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/discarded_event_is_observable/
# from the canonical fixture at
# integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml.
#
# Pipeline:
#   sce-codegen generate <fixture> -o $TMP → auto-emits parent + every
#   inline `<invoke><content>` child as `_sm.rs` (SCE Mesh §9.6.6 rule 2;
#   codegen emit is the single materialization point for synth children)
#   → clear stale `_sm.rs` in the integration tree → copy the new ones in
#   → cargo fmt (pre-commit hook gate) → stitch mod.rs.
#
# Usage (from repo root):
#   scripts/regen_discarded_event_is_observable.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/discarded_event_is_observable"
STEM="discarded_event_is_observable"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Rebuild mod.rs from the actual `_sm.rs` filenames the regen produced.
# Sorted output keeps the diff stable across regens.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_discarded_event_is_observable.sh)"
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
