#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

# Derive the Go module prefix from go.mod so there is a single source
# of truth for the module path. The module root on disk is
# `sce-forge-runtime/go/` (that's where go.mod lives), and every
# generated fixture ends up in `<root>/conformance/generated/<fixture>/`,
# so the prefix sce-codegen has to prepend to each `<sce:import>` is
# `<module>/conformance/generated`. Parsing it out of go.mod means the
# value cannot drift when the module path changes.
GO_MOD_FILE="$REPO_ROOT/sce-forge-runtime/go/go.mod"
GO_MODULE_ROOT="$(awk '$1 == "module" { print $2; exit }' "$GO_MOD_FILE")"
if [[ -z "$GO_MODULE_ROOT" ]]; then
    echo "error: could not parse 'module' directive from $GO_MOD_FILE" >&2
    exit 1
fi
GO_MODULE_PREFIX="$GO_MODULE_ROOT/conformance/generated"

# Rebuild sce-codegen from the current sce-build sources when the Rust
# toolchain is available (local dev). Cargo's incremental build makes this a
# near-instant no-op when nothing has changed; the value is eliminating the
# stale-binary foot-gun where a schema change in conformance.rs would
# otherwise be silently ignored by a test still calling the old binary. In
# CI, the Go job downloads a pre-built artifact from the build-codegen job
# and cargo is not on PATH — this branch is skipped and the existence check
# below catches any misconfiguration.
if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli --release -p sce-build)
fi

if [[ ! -x "$SCE_CODEGEN" ]]; then
    echo "error: sce-codegen binary not found at $SCE_CODEGEN" >&2
    echo "  Build it first: cargo build --bin sce-codegen --features cli --release -p sce-build" >&2
    exit 1
fi

# Pull the fixture list from sce-codegen itself so this script needs neither
# python3 nor jq — the Rust binary owns the manifest schema and prints a
# plain newline-separated list.
#
# `--language go` applies the matrix-aware filter (kind ships per backend;
# per-fixture MCU-only codecs like `sce:dma-burst-align` lower to Rust +
# C11 only and would surface as a `MCU-class kind` error here without
# pre-filtering — same gate the cmake/python/kotlin harnesses use).
FIXTURES=$("$SCE_CODEGEN" list-fixtures \
    --manifest "$MANIFEST" \
    --language go \
    --resource-dir "$RESOURCE_DIR" \
    --format space)

# Clean everything except the gitignore so stale fixtures cannot mask drift.
find "$OUT_DIR" -mindepth 1 -not -name .gitignore -exec rm -rf {} +

for fixture in $FIXTURES; do
    pkg_dir="$OUT_DIR/$fixture"
    mkdir -p "$pkg_dir"
    "$SCE_CODEGEN" generate \
        "$RESOURCE_DIR/$fixture.scxml" \
        --language go \
        --output-dir "$pkg_dir/" \
        --go-module-prefix "$GO_MODULE_PREFIX" >/dev/null
done

# Render the test harness itself from the shared template.
"$SCE_CODEGEN" generate-conformance \
    --language go \
    --manifest "$MANIFEST" \
    --output-dir "$SCRIPT_DIR" >/dev/null

echo "Generated $(echo $FIXTURES | wc -w) Go fixtures and harness under $OUT_DIR"

# Smoke check: every generated package must compile. Byte-goldens
# cannot catch semantic bugs that only surface at `go build` time — see
# feedback memory `byte_goldens_not_compile.md`. Running `go build` on
# the freshly generated tree turns silent drift into a hard error in
# the same script that produced the drift.
echo "Compiling generated Go packages..."
(cd "$REPO_ROOT/sce-forge-runtime/go" && go build ./conformance/...)
echo "go build ./conformance/... OK"
