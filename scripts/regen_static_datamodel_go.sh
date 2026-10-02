#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/static_datamodel/<machine>/ from
# sce-build/tests/fixtures/static_datamodel/<machine>.scxml, for every machine
# the Go backend lowers.
#
# The Go compile+run gate for datamodel="sce-static"
# (docs/SCE_ACCEPTED_SUBSET.md §2.15), the twin of
# scripts/regen_static_datamodel_rust.sh and
# scripts/regen_static_datamodel_kotlin.sh over the same fixtures. The
# committed machines compile as part of `go test ./...` in backends/go/tests, so
# their variables really are fields of the policy and every lowered expression
# — the checked integer operations and the `error.execution` they fail into
# included — really type-checks; static_scenarios_test.go replays the shared
# scenarios against them with no script engine.
#
# A Go package is a directory, so each machine has one of its own.
#
# Usage (from repo root):
#   scripts/regen_static_datamodel_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/static_datamodel"
GENERATED_DIR="backends/go/tests/integration/static_datamodel"

# Derived, not listed: every statechart the fixture directory declares under
# datamodel="sce-static" that the generator lowers for Go — and the ones a test
# keeps beside itself, under tests/integration, which only that test reads. The
# generator is asked, so a construct Go gains the lowering of joins the gate by
# itself. A refusal by name is exit status 7 (generate/unsupported-feature); any
# other failure is a defect, not a machine to skip.
mapfile -t CANDIDATES < <(grep -l 'datamodel="sce-static"' "$INPUT_ROOT"/*.scxml tests/integration/*.scxml | sort)
if [ "${#CANDIDATES[@]}" -lt 1 ]; then
    echo "error: no sce-static statechart found under $INPUT_ROOT" >&2
    exit 1
fi
MACHINES=()
SOURCES=()
for source in "${CANDIDATES[@]}"; do
    status=0
    "$CODEGEN" check -l go "$source" > /dev/null 2>&1 || status=$?
    case "$status" in
        0)
            MACHINES+=("$(basename "$source" .scxml)")
            SOURCES+=("$source")
            ;;
        7) ;;
        *)
            echo "error: sce-codegen check -l go $source failed with status $status" >&2
            exit 1
            ;;
    esac
done
if [ "${#MACHINES[@]}" -lt 1 ]; then
    echo "error: the Go backend lowers no sce-static statechart" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for i in "${!MACHINES[@]}"; do
    "$CODEGEN" generate "${SOURCES[$i]}" -l go -o "$TMP/${MACHINES[$i]}/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} +
for machine in "${MACHINES[@]}"; do
    mkdir -p "$GENERATED_DIR/$machine"
    cp "$TMP/$machine"/*_sm.go "$GENERATED_DIR/$machine/"
done

echo "Regenerated: ${MACHINES[*]} under $GENERATED_DIR/"
