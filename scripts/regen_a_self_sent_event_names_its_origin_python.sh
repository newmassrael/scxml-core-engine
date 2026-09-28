#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/a_self_sent_event_names_its_origin/*_sm.py
# from the canonical fixture at
# integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml.
#
# Mirrors scripts/regen_a_self_sent_event_names_its_origin.sh (Rust). As with
# the other Python integration regens, the generated tree is `.gitignored` and
# CI runs this script before pytest.
#
# Usage (from repo root):
#   scripts/regen_a_self_sent_event_names_its_origin_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml"
GENERATED_DIR="backends/python/tests/integration/a_self_sent_event_names_its_origin"
STEM="a_self_sent_event_names_its_origin"
INPUT_ROOT="integration_resources/a_self_sent_event_names_its_origin"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete

# Normalize the embedded `# From: ${TMP}/...` comment back to the canonical
# fixture path.
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
