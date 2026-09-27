#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/static_datamodel/ from
# sce-build/tests/fixtures/static_datamodel/<machine>.scxml, for every machine
# in MACHINES, and the algorithms they call.
#
# The Rust compile+run gate for datamodel="sce-static"
# (docs/SCE_ACCEPTED_SUBSET.md §2.15), the twin of
# scripts/regen_static_datamodel_kotlin.sh over the same fixtures. The
# committed machines compile as part of `cargo test -p sce-rust-tests`, so
# their variables really are fields of the policy and every lowered
# expression — the checked integer operations and the `error.execution` they
# fail into included — really type-checks; tests/static_datamodel.rs drives
# them with no script engine.
#
# Usage (from repo root):
#   scripts/regen_static_datamodel_rust.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/static_datamodel"
GENERATED_DIR="backends/rust/tests/src/integration/static_datamodel"

# The machines of scripts/regen_static_datamodel_kotlin.sh, for the same
# reasons: variables, typed guards and assignments; a host action with typed
# datamodel arguments; a record variable; a bounded list; an integer
# operation that overflows into error.execution.
# sync_client: a sync run composed of the standard sync rules, driven by
# scenarios/sync_client.json.
MACHINES=(static_counter static_host_call static_record static_list static_overflow sync_client)
# algorithm_days_in_month: called from static_record's guard. A machine's
# import names it `super::<name>`, so it is generated beside the machines.
ALGORITHMS=(days_in_month)
# The standard algorithms sync_client imports, by their library names; each
# is generated beside the machines under its own name, as a local one is.
STD_ALGORITHMS=(sync/sync_failure sync/sync_retry_at sync/sync_delete_outcome sync/sync_upload_outcome)

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for machine in "${MACHINES[@]}"; do
    "$CODEGEN" generate "$INPUT_ROOT/$machine.scxml" -l rust -o "$TMP/"
done
for algorithm in "${ALGORITHMS[@]}"; do
    "$CODEGEN" generate "$INPUT_ROOT/algorithm_$algorithm.scxml" -l rust -o "$TMP/"
done
for algorithm in "${STD_ALGORITHMS[@]}"; do
    "$CODEGEN" generate "sce:std/$algorithm.scxml" -l rust -o "$TMP/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*.rs' ! -name 'mod.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_static_datamodel_rust.sh)"
    echo ""
    for algorithm in "${ALGORITHMS[@]}"; do
        echo "pub mod $algorithm;"
    done
    for algorithm in "${STD_ALGORITHMS[@]}"; do
        echo "pub mod ${algorithm##*/};"
    done
    for machine in "${MACHINES[@]}"; do
        echo "pub mod ${machine}_sm;"
    done
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: ${MACHINES[*]}, ${ALGORITHMS[*]} and ${STD_ALGORITHMS[*]} under $GENERATED_DIR/ from $INPUT_ROOT"
