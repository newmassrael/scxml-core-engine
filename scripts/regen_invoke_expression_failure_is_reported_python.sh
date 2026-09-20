#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/invoke_expression_failure_is_reported/*_sm.py
# from the canonical fixture at
# integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml.
#
# Mirrors scripts/regen_invoke_expression_failure_is_reported.sh (Rust),
# including its child pass: the parent imports the generated hybrid stub
# by module name, so the stub has to be generated or the import fails at
# run time rather than at build time.
#
# As with the other Python integration regens, the generated tree is
# `.gitignored` and CI runs this script before pytest, so the committed
# source tree never carries the SCE-GENERATED Python files.
#
# Usage (from repo root):
#   scripts/regen_invoke_expression_failure_is_reported_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

STEM="invoke_expression_failure_is_reported"
FIXTURE="integration_resources/$STEM/$STEM.scxml"
GENERATED_DIR="backends/python/tests/integration/$STEM"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/"
for child in "$TMP"/*_hybrid*.scxml; do
    [ -e "$child" ] || continue
    "$CODEGEN" generate "$child" -l python -o "$TMP/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
