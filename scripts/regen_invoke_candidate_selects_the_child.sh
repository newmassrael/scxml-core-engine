#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/invoke_candidate_selects_the_child/
# from the canonical fixture at
# integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml.
#
# The children here are the documents `sce:candidates` declares — real
# machines, not the immediate-`<final>` stub a hybrid invoke gets when it
# declares none (docs/SCE_ACCEPTED_SUBSET.md §2.13). Codegen stages them
# beside the parent's output, and each is generated as a child so the
# parent's reference to it resolves.
#
# Usage (from repo root):
#   scripts/regen_invoke_candidate_selects_the_child.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
STEM="invoke_candidate_selects_the_child"
FIXTURE="integration_resources/$STEM/$STEM.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/$STEM"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"
for child in "$TMP"/*.scxml; do
    [ -e "$child" ] || continue
    case "$(basename "$child")" in
        "$STEM.scxml") continue ;;
    esac
    "$CODEGEN" generate "$child" --as-child --parent-stem "$STEM" -l rust -o "$TMP/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_$STEM.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
    while IFS= read -r child_stem; do
        echo "mod ${child_stem};"
        echo "pub use ${child_stem}::*;"
    done < <(
        find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -printf '%f\n' \
            | sed 's/\.rs$//' | grep -v "^${STEM}_sm$" | sort
    )
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
