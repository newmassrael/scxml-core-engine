#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Write each named tree into this checkout as its declared actions produce it
# (ADR 0097): `sce-codegen materialize <tree> --into <checkout> --replace`, so
# a file the actions no longer produce is removed too. A tree is declared in
# tools/codegen/trees/<tree>.toml; an action whose inputs are unchanged is
# answered by the action cache without running the generator.
#
# This is the one command for a declared tree, for a person and for a
# mutation casefile's `mutation_regen` alike, so neither keeps a script of
# its own per document.
#
# Usage (from anywhere in the checkout):
#   scripts/materialize.sh <tree>...
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: scripts/materialize.sh <tree>... (the trees are tools/codegen/trees/*.toml)" >&2
    exit 2
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

for tree in "$@"; do
    "$CODEGEN" materialize "$tree" --into "$REPO_ROOT" --replace
done
