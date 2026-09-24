#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/<machine>/
# from sce-build/tests/fixtures/static_datamodel/<machine>.scxml, for every
# machine in MACHINES.
#
# The Kotlin compile+run gate for datamodel="sce-static"
# (docs/SCE_ACCEPTED_SUBSET.md §2.15). The committed machines compile as part
# of `:sce-kotlin-tests`, so their variables really are Kotlin fields and every
# lowered expression really type-checks; StaticDatamodelTest.kt drives them
# with no script engine. Driven by its own script rather than the
# `generate-integration` fan-out because only Kotlin lowers the model today —
# the other backends refuse the document (`STATIC_DATAMODEL_BACKENDS`).
#
# Usage (from repo root):
#   scripts/regen_static_datamodel_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/static_datamodel"
GENERATED_ROOT="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration"
PACKAGE_PREFIX="com.sce.integration"

# static_counter:   variables, a typed guard and assignments.
# static_host_call: a native host action taking typed datamodel arguments.
# static_record:    a record variable built whole and updated field by field,
#                   whose schema is imported from a sibling document.
# static_list:      a list variable filled by <sce:append>, emptied by
#                   <sce:clear>, and held to its capacity.
MACHINES=(static_counter static_host_call static_record static_list)

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for machine in "${MACHINES[@]}"; do
    "$CODEGEN" generate "$INPUT_ROOT/$machine.scxml" -l kotlin -o "$TMP/$machine/" \
        --input-root "$INPUT_ROOT" \
        --kotlin-package-prefix "$PACKAGE_PREFIX"
    dir="$GENERATED_ROOT/$machine"
    mkdir -p "$dir"
    find "$dir" -maxdepth 1 -name '*Sm.kt' -delete
    for src in "$TMP/$machine"/*Sm.kt; do
        [[ -f "$src" ]] || continue
        sed -i "s|// Source: ${TMP}/${machine}/|// Source: ${INPUT_ROOT}/|g" "$src"
        cp "$src" "$dir/"
    done
done

# The algorithms a machine above imports. `sce-codegen generate` does not
# generate a document's imports (a forge kind's are generated on their own
# too), and an algorithm's package is `com.sce.generated.<snake>` whatever
# the machine's prefix — the package the machine's import line names.
ALGORITHM_ROOT="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/generated"
# algorithm_days_in_month: called from static_record's guard.
ALGORITHMS=(days_in_month)
for algorithm in "${ALGORITHMS[@]}"; do
    "$CODEGEN" generate "$INPUT_ROOT/algorithm_$algorithm.scxml" -l kotlin -o "$TMP/algorithm_$algorithm/"
    dir="$ALGORITHM_ROOT/$algorithm"
    mkdir -p "$dir"
    find "$dir" -maxdepth 1 -name '*.kt' -delete
    cp "$TMP/algorithm_$algorithm"/*.kt "$dir/"
done

echo "Regenerated: ${MACHINES[*]} under $GENERATED_ROOT/ and ${ALGORITHMS[*]} under $ALGORITHM_ROOT/ from $INPUT_ROOT"
