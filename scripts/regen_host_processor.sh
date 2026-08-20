#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/host_processor/ from the
# canonical W3C SCXML 6.2.5 host-served-processor fixture at
# sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml.
#
# This is the Rust compile+run gate for a `<send type>` the HOST serves. The
# `--host-processor` declaration below is the whole point of the script: the
# same fixture generated WITHOUT it compiles to a W3C SCXML 6.2
# error.execution, and with it compiles to a dispatch into the registry the
# host registers a handler in. Because the tree is part of the crate it is
# really type-checked, and the runtime test (tests/host_processor.rs) drives
# the machine twice off one binary — once with a handler registered, once
# without — so the pair measures the registration rather than the build.
#
# Rust-only. The other backends have no host-processor runtime registry, and
# the generator refuses the declaration for them by name
# (`reject_host_processors_in_unsupported_lang`) rather than emitting a
# dispatch nothing can service. That refusal is what makes a Rust-only gate
# honest instead of a coverage gap: a declaration cannot silently compile on
# a backend that would drop it. Called directly from
# regen_all_committed_trees.sh rather than through the
# `generate-integration` fan-out, which has no per-stem flags — the same
# reason regen_native_action.sh is called directly.
#
# Usage (from repo root):
#   scripts/regen_host_processor.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/host_processor"

# The declared type. Kept in one place here and asserted from the runtime
# test, so the string the build was given and the string the host registers
# cannot drift into two.
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*_sm.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_host_processor.sh)"
    echo ""
    echo "mod statechart_host_processor_sm;"
    echo "pub use statechart_host_processor_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
