#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/IntegrationMachines/ from the canonical fixtures of
# integration_resources/, for every stem the Swift backend lowers.
#
# The Swift channel of the integration fixtures, the twin of the per-stem
# `scripts/regen_<stem>_python.sh` scripts and of the Kotlin tree that
# tools/codegen/trees/kotlin-integration.toml declares. Swift has one script for every stem
# instead of a generation each, because what a Kotlin generation needs is a package of its own
# per stem, with its children joining it (`--children`, `--kotlin-package-prefix`), to keep the
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

# The `<send>` and `<invoke>` types the host of a stem's driver serves, declared to its generation
# as the C++ fixture registration declares them (`HOST_PROCESSOR` / `HOST_INVOKER` of
# `sce_generate_static_integration_test` in tests/CMakeLists.txt) and as the stem's Python script
# passes them: the build decides whether a `<send type>` or an `<invoke type>` dispatches to the
# host or refuses, so a stem whose document addresses the host is generated with the types its
# driver registers. One line per argument, for `mapfile`; none for a stem the host serves nothing.
host_args() {
    case "$1" in
        an_event_no_processor_delivered_has_no_origin) printf '%s\n' --host-processor x-sce-host ;;
        a_host_names_its_origin_in_its_own_terms) printf '%s\n' --host-invoker x-sce-host ;;
    esac
}

# The canonical document of a stem is GENERATED to learn whether the backend lowers it, and not
# `check`ed: `check -l swift` does not see every refusal (`flat_lifecycle` passes it and `generate`
# refuses its transition function), so the verdict that counts is the one that writes the file.
LOWERED=()
REFUSED=()
for stem in "${STEMS[@]}"; do
    status=0
    mapfile -t HOST_ARGS < <(host_args "$stem")
    verdict="$("$CODEGEN" generate "$INPUT_ROOT/$stem/$stem.scxml" -l swift --error-format=json -o "$TMP/$stem/" "${HOST_ARGS[@]}" 2>&1)" || status=$?
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
    # The stub a hybrid `<invoke>` that declares no candidates starts (W3C SCXML 6.4): `generate`
    # writes it, for every backend, beside the machine it belongs to, and the machine of the stub is
    # the child the parent instantiates. Matched by the name the build gives it, so a child the
    # stem carries and `generate` copied there is not generated twice. Generated from a path
    # relative to the temp root: the machine's header names its source document, and an absolute
    # path under a directory `mktemp` chose would put a different line in the committed file on
    # every run. The generator finds its templates beside its source tree or under the directory it
    # runs in, and it runs in the temp root here, so the directory is named.
    for stub in "$TMP/$stem"/*_hybrid[0-9]*.scxml; do
        [ -e "$stub" ] || continue
        (cd "$TMP" && SCE_TEMPLATE_DIR="$REPO_ROOT/tools/codegen/templates" \
            "$CODEGEN" generate "$stem/$(basename "$stub")" -l swift -o "$stem/")
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
    # A machine, and for a document that declares `sce:transition-function="emit"` the function
    # beside it (SCE Accepted Subset §2.21): files of one SwiftPM target, named for the document.
    for generated in "$TMP/$stem"/*_sm.swift "$TMP/$stem"/*_transition_function.swift; do
        [ -e "$generated" ] || continue
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
