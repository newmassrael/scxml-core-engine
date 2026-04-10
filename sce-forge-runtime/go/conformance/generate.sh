#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Regenerate the Go cross-language conformance artifacts from the shared
# fixture catalog. Two codegen passes, both invoking sce-codegen on
# tests/forge/conformance/fixtures.json:
#
#   1. `generate` per fixture — writes one Go package per SCXML under
#      ./generated/<name>/<name>.go. The package-per-directory layout is a
#      Go language requirement.
#
#   2. `generate-conformance --language go` — renders the test harness
#      itself from the shared template
#      tools/codegen/templates/forge/go/conformance/harness.go.jinja2.
#      The output overwrites numerical_conformance_test.go in this
#      directory. No test scaffolding is hand-maintained.
#
# Adding a fixture means: drop the SCXML under tests/forge/resources/, add
# an entry to fixtures.json and numerical_reference.json, and rerun this
# script.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SCE_CODEGEN="$REPO_ROOT/target/release/sce-codegen"
RESOURCE_DIR="$REPO_ROOT/tests/forge/resources"
MANIFEST="$REPO_ROOT/tests/forge/conformance/fixtures.json"
OUT_DIR="$SCRIPT_DIR/generated"

if [[ ! -x "$SCE_CODEGEN" ]]; then
    echo "error: sce-codegen binary not found at $SCE_CODEGEN" >&2
    echo "  Build it first: cargo build --bin sce-codegen --features cli --release -p sce-build" >&2
    exit 1
fi

# Pull the fixture list from sce-codegen itself so this script needs neither
# python3 nor jq — the Rust binary owns the manifest schema and prints a
# plain newline-separated list.
FIXTURES=$("$SCE_CODEGEN" list-fixtures --manifest "$MANIFEST" --format space)

# Clean everything except the gitignore so stale fixtures cannot mask drift.
find "$OUT_DIR" -mindepth 1 -not -name .gitignore -exec rm -rf {} +

for fixture in $FIXTURES; do
    pkg_dir="$OUT_DIR/$fixture"
    mkdir -p "$pkg_dir"
    "$SCE_CODEGEN" generate \
        "$RESOURCE_DIR/$fixture.scxml" \
        --language go \
        --output-dir "$pkg_dir/" >/dev/null
done

# Render the test harness itself from the shared template.
"$SCE_CODEGEN" generate-conformance \
    --language go \
    --manifest "$MANIFEST" \
    --output-dir "$SCRIPT_DIR" >/dev/null

echo "Generated $(echo $FIXTURES | wc -w) Go fixtures and harness under $OUT_DIR"
