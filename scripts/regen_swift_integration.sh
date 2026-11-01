#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/IntegrationMachines/ from the canonical fixtures of
# integration_resources/, for every stem the Swift backend lowers.
#
# The Swift channel of the integration fixtures, the twin of the per-stem
# `scripts/regen_<stem>_kotlin.sh` scripts and of `generate-integration -l kotlin` that runs
# them. Swift has one script for every stem instead of one per stem, because what a stem script
# does for Kotlin is staging the fixture so synth-invoke children land outside the canonical
# root, parent generate, per-child generate with `--parent-stem` and a package prefix to keep the
# integration and W3C package roots apart; and Swift needs none of it. `generate -l swift` writes a
# child's machine next to its parent's, and every type is named for its document, so the
# machines of every stem are files of ONE SwiftPM target without a namespace of their own.
#
# Derived, not listed: every stem the fixture root holds whose canonical document the generator
# lowers for Swift. The generator is asked, so a construct Swift gains the lowering of joins the
# channel by itself. A refusal by name is exit status 7 (generate/unsupported-feature) and is
# printed at the end with its reason: it stands in `channel_cannot_host` as an exemption until it
# is built, and `a_swift_integration_exemption_still_holds` holds the two to each other. Any other
# failure is a defect, not a stem to skip.
#
# A stem's other documents (the children a document names by `src`) are generated with it when
# its canonical document is.
#
# Usage (from repo root):
#   scripts/regen_swift_integration.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="integration_resources"
GENERATED_DIR="backends/swift/tests/Sources/IntegrationMachines"

mapfile -t STEMS < <(find "$INPUT_ROOT" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | LC_ALL=C sort)
if [ "${#STEMS[@]}" -lt 1 ]; then
    echo "error: no stem found under $INPUT_ROOT" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# The canonical document of a stem is GENERATED to learn whether the backend lowers it, and not
# `check`ed: `check -l swift` does not see every refusal (`flat_lifecycle` passes it and `generate`
# refuses its transition function), so the verdict that counts is the one that writes the file.
LOWERED=()
REFUSED=()
for stem in "${STEMS[@]}"; do
    status=0
    verdict="$("$CODEGEN" generate "$INPUT_ROOT/$stem/$stem.scxml" -l swift --error-format=json -o "$TMP/$stem/" 2>&1)" || status=$?
    case "$status" in
        0) LOWERED+=("$stem") ;;
        7)
            rm -rf "${TMP:?}/$stem"
            REFUSED+=("$stem: $(printf '%s' "$verdict" | grep -o '"message":"[^"]*"' | head -n 1 | cut -c1-170)")
            ;;
        *)
            echo "error: sce-codegen generate -l swift $INPUT_ROOT/$stem/$stem.scxml failed with status $status" >&2
            exit 1
            ;;
    esac
done
if [ "${#LOWERED[@]}" -lt 1 ]; then
    echo "error: the Swift backend lowers no integration stem" >&2
    exit 1
fi

# The rest of a stem's documents, the children it names by `src`, are generated with it.
for stem in "${LOWERED[@]}"; do
    for document in "$INPUT_ROOT/$stem"/*.scxml; do
        [ "$document" = "$INPUT_ROOT/$stem/$stem.scxml" ] && continue
        "$CODEGEN" generate "$document" -l swift -o "$TMP/$stem/"
    done
done

mkdir -p "$GENERATED_DIR"
# Every file below is generated and the tests beside them are not: clearing by what each file says
# it is takes the first and leaves the second.
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear --recursive "$GENERATED_DIR"
# A machine is named for its document, and the machines are files of one SwiftPM target, so two
# documents of one name would be one file that holds whichever was copied last: a defect of the
# documents, not something to order.
declare -A MACHINE_FROM=()
for stem in "${LOWERED[@]}"; do
    for generated in "$TMP/$stem"/*_sm.swift; do
        unit="$(basename "$generated")"
        if [ -n "${MACHINE_FROM[$unit]:-}" ]; then
            echo "error: ${MACHINE_FROM[$unit]} and $stem are both written as $unit; one of them needs another name" >&2
            exit 1
        fi
        cp "$generated" "$GENERATED_DIR/"
        MACHINE_FROM[$unit]="$stem"
    done
done

echo "Regenerated: ${LOWERED[*]} under $GENERATED_DIR/"
if [ "${#REFUSED[@]}" -gt 0 ]; then
    echo "Not lowered yet (${#REFUSED[@]}), refused by name:"
    printf '  %s\n' "${REFUSED[@]}"
fi
