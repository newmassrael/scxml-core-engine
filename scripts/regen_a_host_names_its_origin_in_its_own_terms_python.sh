#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/a_host_names_its_origin_in_its_own_terms/*_sm.py
# from the canonical fixture at
# integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml.
#
# The document has the host run an `<invoke>`, so the build is told the host
# runs `x-sce-host` (`--host-invoker`), as the C++ fixture registration is
# (`HOST_INVOKER x-sce-host`): without it the invoke compiles to the
# error.execution of a type nobody runs, and the driver would measure that
# instead of the ending. As with the other Python integration regens, the
# generated tree is `.gitignored` and CI runs this script (through
# `generate-integration -l python`) before pytest.
#
# Usage (from repo root):
#   scripts/regen_a_host_names_its_origin_in_its_own_terms_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml"
GENERATED_DIR="backends/python/tests/integration/a_host_names_its_origin_in_its_own_terms"
STEM="a_host_names_its_origin_in_its_own_terms"
INPUT_ROOT="integration_resources/a_host_names_its_origin_in_its_own_terms"

# Kept in one place here and asserted from the driver, so the string the build
# was given and the string the host registers cannot drift into two.
HOST_INVOKER="x-sce-host"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT" --host-invoker "$HOST_INVOKER"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"

# Normalize the embedded `# From: ${TMP}/...` comment back to the canonical
# fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-invoker $HOST_INVOKER)"
