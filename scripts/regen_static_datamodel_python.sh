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
# own. A document is named as the generator takes it — a local one by its path, a
# standard one by its library name (`sce:std/...`) — and read for its own imports
# where it lies: a standard one under stdlib/.

# The file a document named as the generator takes it is read from.
document_file() {
    case "$1" in
        sce:std/*) printf '%s\n' "stdlib/${1#sce:std/}" ;;
        *) printf '%s\n' "$1" ;;
    esac
}

# The documents of kind $1 that document $2 imports, each named as the generator
# takes it: a standard one as it is written, any other beside the document that
# imports it — which, for a standard document, is the library again.
imports_of() {
    local kind="$1" document="$2" import
    while IFS= read -r import; do
        case "$import" in
            sce:*) printf '%s\n' "$import" ;;
            *) printf '%s\n' "$(dirname "$document")/$import" ;;
        esac
    done < <(grep "<sce:import kind=\"$kind\"" "$(document_file "$document")" | grep -o 'src="[^"]*"' | cut -d'"' -f2)
}

# Every algorithm a machine calls, and every algorithm one of those calls in its
# turn: an algorithm's module imports each it calls as a sibling, as a machine's
# does.
ALGORITHMS=()
declare -A REACHED=()
PENDING=()
for source in "${SOURCES[@]}"; do
    while IFS= read -r algorithm; do
        PENDING+=("$algorithm")
    done < <(imports_of algorithm "$source")
done
while [ "${#PENDING[@]}" -gt 0 ]; do
    algorithm="${PENDING[0]}"
    PENDING=("${PENDING[@]:1}")
    if [ -n "${REACHED[$algorithm]:-}" ]; then
        continue
    fi
    REACHED[$algorithm]=1
    ALGORITHMS+=("$algorithm")
    while IFS= read -r called; do
        PENDING+=("$called")
    done < <(imports_of algorithm "$algorithm")
done
if [ "${#ALGORITHMS[@]}" -gt 0 ]; then
    mapfile -t ALGORITHMS < <(printf '%s\n' "${ALGORITHMS[@]}" | sort -u)
fi
# The records an algorithm takes or returns are the payload classes of the
# event-schemas it imports, which its module imports as siblings, and so does the
# machine that holds one. Derived from the algorithms, as they are from the
# machines, for the same reason, a standard one's as a local one's: generating a
# standard algorithm generates none of what it imports.
SCHEMAS=()
for algorithm in "${ALGORITHMS[@]}"; do
    while IFS= read -r schema; do
        SCHEMAS+=("$schema")
    done < <(imports_of event-schema "$algorithm")
done
if [ "${#SCHEMAS[@]}" -gt 0 ]; then
    mapfile -t SCHEMAS < <(printf '%s\n' "${SCHEMAS[@]}" | sort -u)
fi
# The enums those records and algorithms hold a value of: a record's enum field and
# an algorithm's enum slot are of the enum document's own class, a sibling module
# the schema, the algorithm and the adapter a machine maps its own members through
# import (SCE Accepted Subset §2.15).
ENUMS=()
for document in "${SCHEMAS[@]}" "${ALGORITHMS[@]}"; do
    while IFS= read -r enum; do
        ENUMS+=("$enum")
    done < <(imports_of enum "$document")
done
if [ "${#ENUMS[@]}" -gt 0 ]; then
    mapfile -t ENUMS < <(printf '%s\n' "${ENUMS[@]}" | sort -u)
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for i in "${!MACHINES[@]}"; do
    "$CODEGEN" generate "${SOURCES[$i]}" -l python -o "$TMP/${MACHINES[$i]}/"
done
for i in "${!ALGORITHMS[@]}"; do
    "$CODEGEN" generate "${ALGORITHMS[$i]}" -l python -o "$TMP/algorithm_$i/"
done
for i in "${!SCHEMAS[@]}"; do
    "$CODEGEN" generate "${SCHEMAS[$i]}" -l python -o "$TMP/schema_$i/"
done
for i in "${!ENUMS[@]}"; do
    "$CODEGEN" generate "${ENUMS[$i]}" -l python -o "$TMP/enum_$i/"
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

for i in "${!SCHEMAS[@]}"; do
    for generated in "$TMP/schema_$i"/*.py; do
        cp "$generated" "$GENERATED_DIR/"
        MODULES+=("$(basename "$generated" .py)")
    done
done

for i in "${!ENUMS[@]}"; do
    for generated in "$TMP/enum_$i"/*.py; do
        cp "$generated" "$GENERATED_DIR/"
        MODULES+=("$(basename "$generated" .py)")
    done
done

echo "Regenerated: ${MACHINES[*]} and the algorithms ${MODULES[*]:-none} under $GENERATED_DIR/"
