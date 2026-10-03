#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/static_datamodel/<machine>_sm.py
# from sce-build/tests/fixtures/static_datamodel/<machine>.scxml, for every
# machine the Python backend lowers.
#
# The Python run gate for datamodel="sce-static"
# (docs/SCE_ACCEPTED_SUBSET.md §2.15), the twin of
# scripts/regen_static_datamodel_go.sh over the same fixtures:
# test_static_scenarios.py replays the shared scenarios against the generated
# machines, whose variables really are attributes of the policy and whose every
# lowered statement really runs with no datamodel in a script engine.
#
# Unlike the Rust / Kotlin / Go trees, the Python one is gitignored
# (`backends/python/tests/integration/*/*_sm.py`), so nothing here is committed:
# scripts/gates/w3c-python.sh runs this script before pytest, and so does a
# developer after a fixture or template edit.
#
# Usage (from repo root):
#   scripts/regen_static_datamodel_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/static_datamodel"
GENERATED_DIR="backends/python/tests/integration/static_datamodel"

# Derived, not listed: every statechart the fixture directory declares under
# datamodel="sce-static" that the generator lowers for Python — and the ones a
# test keeps beside itself, under tests/integration, which only that test reads.
# The generator is asked, so a construct Python gains the lowering of joins the
# gate by itself. A refusal by name is exit status 7
# (generate/unsupported-feature); any other failure is a defect, not a machine
# to skip.
mapfile -t CANDIDATES < <(grep -l 'datamodel="sce-static"' "$INPUT_ROOT"/*.scxml tests/integration/*.scxml | sort)
if [ "${#CANDIDATES[@]}" -lt 1 ]; then
    echo "error: no sce-static statechart found under $INPUT_ROOT" >&2
    exit 1
fi
MACHINES=()
SOURCES=()
for source in "${CANDIDATES[@]}"; do
    status=0
    "$CODEGEN" check -l python "$source" > /dev/null 2>&1 || status=$?
    case "$status" in
        0)
            MACHINES+=("$(basename "$source" .scxml)")
            SOURCES+=("$source")
            ;;
        7) ;;
        *)
            echo "error: sce-codegen check -l python $source failed with status $status" >&2
            exit 1
            ;;
    esac
done
if [ "${#MACHINES[@]}" -lt 1 ]; then
    echo "error: the Python backend lowers no sce-static statechart" >&2
    exit 1
fi

# Derived from what the lowered machines import, for the reason the machines
# are: a list here is one more place a new algorithm has to be remembered, and
# one left out is a machine that imports a module nothing generated. Each
# algorithm a machine calls is generated beside the machines as a module of its
# own — a local one is named by its path beside the machine, a standard one by
# its library name (`sce:std/...`).
ALGORITHMS=()
for source in "${SOURCES[@]}"; do
    while IFS= read -r import; do
        case "$import" in
            sce:*) ALGORITHMS+=("$import") ;;
            *) ALGORITHMS+=("$(dirname "$source")/$import") ;;
        esac
    done < <(grep 'kind="algorithm"' "$source" | grep -o 'src="[^"]*"' | cut -d'"' -f2)
done
if [ "${#ALGORITHMS[@]}" -gt 0 ]; then
    mapfile -t ALGORITHMS < <(printf '%s\n' "${ALGORITHMS[@]}" | sort -u)
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for i in "${!MACHINES[@]}"; do
    "$CODEGEN" generate "${SOURCES[$i]}" -l python -o "$TMP/${MACHINES[$i]}/"
done
for i in "${!ALGORITHMS[@]}"; do
    "$CODEGEN" generate "${ALGORITHMS[$i]}" -l python -o "$TMP/algorithm_$i/"
done

# What this directory holds that is not generated is its package marker and its
# test: a machine imports an algorithm as a sibling module (`from . import x`),
# as a forge kind imports another, so the machines and the algorithms are one
# package.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*.py' ! -name '__init__.py' ! -name 'test_*.py' -delete
for machine in "${MACHINES[@]}"; do
    cp "$TMP/$machine"/*_sm.py "$GENERATED_DIR/"
done
MODULES=()
for i in "${!ALGORITHMS[@]}"; do
    for generated in "$TMP/algorithm_$i"/*.py; do
        cp "$generated" "$GENERATED_DIR/"
        MODULES+=("$(basename "$generated" .py)")
    done
done

echo "Regenerated: ${MACHINES[*]} and the algorithms ${MODULES[*]:-none} under $GENERATED_DIR/"
