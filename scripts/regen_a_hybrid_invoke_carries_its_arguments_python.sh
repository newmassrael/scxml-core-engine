#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/a_hybrid_invoke_carries_its_arguments/*_sm.py
# from the canonical fixture at
# integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml.
#
# Mirrors scripts/regen_a_hybrid_invoke_carries_its_arguments.sh (Rust). The
# parent imports each candidate module by name, so every declared candidate
# has to be generated or the selection fails at run time rather than at
# build time.
#
# As with the other Python integration regens, the generated tree is
# `.gitignored` and CI runs this script before pytest.
#
# Usage (from repo root):
#   scripts/regen_a_hybrid_invoke_carries_its_arguments_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

STEM="a_hybrid_invoke_carries_its_arguments"
FIXTURE_DIR="integration_resources/$STEM"
FIXTURE="$FIXTURE_DIR/$STEM.scxml"
GENERATED_DIR="backends/python/tests/integration/$STEM"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/"
# ⚠ Generated from the TRACKED document, not from the copy the parent run
# staged beside itself, so the `# From:` line of a generated artefact names
# a path that is the same on every run.
for child in "$TMP"/*.scxml; do
    [ -e "$child" ] || continue
    base="$(basename "$child")"
    [ "$base" = "$STEM.scxml" ] && continue
    src="$FIXTURE_DIR/$base"
    [ -f "$src" ] || src="$child"
    "$CODEGEN" generate "$src" -l python -o "$TMP/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
