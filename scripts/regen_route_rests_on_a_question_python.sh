#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/route_rests_on_a_question/*_sm.py
# from sce-build/tests/fixtures/route_rests_on_a_question/route_rests_on_a_question.scxml.
#
# The Python compile+run gate for a `<send>` whose route (`targetexpr`) is read
# from data the specification leaves open (W3C SCXML 6.2.4): the generated
# machine tells the engine which questions the route rests on, and the engine
# gives them back with the error that send raised and nothing answered. Only the
# Python runtime holds this, because the host that plays a design to an owner is
# the Python authoring driver; the fixture is therefore not a stem under
# `integration_resources/`, where a stem is a seven-channel commitment.
#
# The generated module is gitignored like every other module in
# `backends/python/tests/integration/`; the `w3c-python` gate regenerates it, and
# this script is for a local run.
#
# Usage (from repo root):
#   scripts/regen_route_rests_on_a_question_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/route_rests_on_a_question/route_rests_on_a_question.scxml"
INPUT_ROOT="sce-build/tests/fixtures/route_rests_on_a_question"
GENERATED_DIR="backends/python/tests/integration/route_rests_on_a_question"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/" --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
