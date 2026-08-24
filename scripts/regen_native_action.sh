#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/native_action/ from the canonical
# W3C SCXML G.7 `<sce:action>` fixture at
# sce-build/tests/fixtures/event_schema/statechart_native_action.scxml.
#
# This regenerates the three backends that COMMIT a tree for this fixture:
# Rust, Go and Kotlin (Python has its own script for the reason given there;
# C++ and C11 generate at build time from their own CMake registrations).
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
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="backends/rust/tests/src/integration/native_action"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
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

# The Go half. Named for the document's own stem rather than for the axis,
# unlike the Rust tree above: a Go package name comes from the emitted file, so
# a directory named anything else puts two package names in one directory and
# the module stops building.
#
# The `// From:` rewrite is the same one every `regen_*_go.sh` does — the
# tracked artefact has to point at the canonical fixture rather than at the
# temporary directory this run happened to use, or the tree stops reproducing
# on another machine.
GO_GENERATED_DIR="backends/go/tests/integration/statechart_native_action"
GO_TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$GO_TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$GO_TMP/" --input-root "$INPUT_ROOT"

mkdir -p "$GO_GENERATED_DIR"
find "$GO_GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete
for src in "$GO_TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${GO_TMP}/|// From: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GO_GENERATED_DIR/"
done

# The Kotlin half. Its own directory because a Kotlin package is a directory,
# the same constraint the Go half states above.
KT_GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/statechart_native_action"
KT_PACKAGE_PREFIX="com.sce.integration"
KT_TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$GO_TMP" "$KT_TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$KT_TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$KT_PACKAGE_PREFIX"

mkdir -p "$KT_GENERATED_DIR"
find "$KT_GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
for src in "$KT_TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${KT_TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$KT_GENERATED_DIR/"
done

# The Python half is NOT here, and that is deliberate:
# `backends/python/tests/integration/*/*_sm.py` is gitignored, so there is no
# committed artefact for this script to keep current. `scripts/gates/
# w3c-python.sh` calls `scripts/regen_native_action_python.sh` instead — the
# same split `regen_host_processor{,_python}.sh` already makes.
echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
echo "Regenerated: $GO_GENERATED_DIR/ from $FIXTURE"
echo "Regenerated: $KT_GENERATED_DIR/ from $FIXTURE"
