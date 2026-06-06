#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-rust-tests/src/integration/native_action/ from the canonical
# W3C SCXML 5.11 `<sce:action>` fixture at
# sce-build/tests/fixtures/event_schema/statechart_native_action.scxml.
#
# This is the Rust compile+run gate for native host-trait action dispatch:
# the committed SM declares an `<Machine>Actions` trait and a generic
# `Policy<A>` with NO IScriptEngine. Because the tree is part of the crate it
# is REALLY type-checked (unlike the syntax-only smoke gate), and the runtime
# test (tests/native_action.rs) drives a host `Actions` impl and asserts the
# side effects fired. Rust-only for now — the other backends do not yet lower
# `<sce:action>` natively.
#
# Usage (from repo root):
#   scripts/regen_native_action.sh
#
# Requires:
#   target/release/sce-codegen (auto-built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_native_action.scxml"
GENERATED_DIR="sce-rust-tests/src/integration/native_action"

if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

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

cargo fmt -p sce-rust-tests

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
