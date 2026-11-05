#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/an_event_no_processor_delivered_has_no_origin/*_sm.py
# from the canonical fixture at
# integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml.
#
# The document sends to the host's processor, so the build is told the host
# serves `x-sce-host` (`--host-processor`), as the C++ fixture registration is
# (`HOST_PROCESSOR x-sce-host`): without it the send compiles to the
# error.execution of a type nobody serves, and the driver would measure that
# instead of the reply. As with the other Python integration regens, the
# generated tree is `.gitignored` and CI runs this script (through
# `generate-integration -l python`) before pytest.
#
# Usage (from repo root):
#   scripts/regen_an_event_no_processor_delivered_has_no_origin_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml"
GENERATED_DIR="backends/python/tests/integration/an_event_no_processor_delivered_has_no_origin"
STEM="an_event_no_processor_delivered_has_no_origin"
INPUT_ROOT="integration_resources/an_event_no_processor_delivered_has_no_origin"

# Kept in one place here and asserted from the driver, so the string the build
# was given and the string the host registers cannot drift into two.
HOST_PROCESSOR="x-sce-host"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cp "$FIXTURE" "$TMP/$STEM.scxml"

"$CODEGEN" generate "$TMP/$STEM.scxml" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT" --host-processor "$HOST_PROCESSOR"

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

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
