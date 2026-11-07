#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/native_action/ from the canonical
# W3C SCXML G.7 `<sce:action>` fixture at
# sce-build/tests/fixtures/event_schema/statechart_native_action.scxml.
#
# Three backends COMMIT a tree for this fixture, each from a script of its own:
# this one for Rust, `regen_native_action_go.sh` for Go and
# `regen_native_action_kotlin.sh` for Kotlin (Python has its own script for the
# reason given there; C++ and C11 generate at build time from their own CMake
# registrations).
#
# One script wrote all three trees until 2026-11-07. It was split because the
# regeneration runs as one bx action per language tree (bx.toml, the
# `regen-*` actions): a script that writes three trees belongs to no one action,
# and an action is refused when it writes outside the outputs it declares.
#
# Each committed SM declares a `<Machine>Actions` host interface and takes an
# implementation of it where the machine is constructed, with NO script engine
# anywhere in the emitted code. Because each tree is part of its language's
# test module it is REALLY compiled (unlike a syntax-only smoke gate), and the
# runtime test beside it drives a host implementation and asserts the side
# effects fired with the typed arguments the event carried.
#
# Six backends, one document. §scxml-G-7 was Rust-only until 2026-08-24, and
# the refusal every other backend raised said so in its own message — a
# backend-coverage gap the refusal kept honest rather than silent. What closed
# it is one lowering (`forge::native_action::render`) that each backend spells
# in its own convention, so the six cannot drift into six lowerings.
#
# Usage (from repo root):
#   scripts/regen_native_action.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_native_action.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/native_action"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
cp "$TMP"/*_sm.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_native_action.sh)"
    echo ""
    echo "mod statechart_native_action_sm;"
    echo "pub use statechart_native_action_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
