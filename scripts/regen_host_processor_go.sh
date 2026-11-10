#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Go trees of the host-served Event I/O Processor and host-run
# invoke fixtures under sce-build/tests/fixtures/host_processor/, one directory
# under backends/go/tests/integration/ for each document.
#
# The Go half of the host-processor generation whose Rust half
# `tools/codegen/trees/rust-integration.toml` declares; each document's own
# comment says what it measures. It was part of the Rust script until
# 2026-11-07. It is a script of its own because the regeneration runs as one bx
# action per language tree (bx.toml, the `regen-*` actions), and a script that
# writes the Rust and the Go trees belongs to no one action.
#
# The Go runtime carries a `<send>` registry (§scxml-6.2.5) and an `<invoke>` one
# (§scxml-6.4.1), so no document here meets a refusal. Each lands in a directory
# named for the document's own stem rather than for the axis: a Go package name
# comes from the emitted file, so two stems in one directory are two package
# names in one directory and the module stops building.
#
# The `// From:` rewrite is the same one every `regen_*_go.sh` does — the
# tracked artefact has to point at the canonical fixture rather than at the
# temporary directory this run happened to use, or the tree stops reproducing
# on another machine.
#
# Usage (from repo root):
#   scripts/regen_host_processor_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"

# The declared types, the same strings
# `tools/codegen/trees/rust-integration.toml` gives the Rust trees and the
# runtime tests register.
HOST_PROCESSOR="x-sce-host"
HOST_INVOKER="x-sce-host"

FIXTURES="sce-build/tests/fixtures/host_processor"
GO_ROOT="backends/go/tests/integration"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# The Go tree of the document STEM under FIXTURES, generated with the
# declarations that follow, replacing the generated files of its directory.
generate_go() {
    local stem="$1" fixture out src dir
    shift
    fixture="$FIXTURES/$stem.scxml"
    dir="$GO_ROOT/$stem"
    out="$(mktemp -d -p "$TMP")"
    "$CODEGEN" generate "$fixture" -l go -o "$out/" "$@"
    mkdir -p "$dir"
    sce_generated_tree_clear "$dir"
    for src in "$out"/*_sm.go; do
        [[ -f "$src" ]] || continue
        sed -i "s|// From: ${out}/|// From: $FIXTURES/|g" "$src"
    done
    cp "$out"/*_sm.go "$dir/"
    echo "Regenerated: $dir/ from $fixture $*"
}

generate_go statechart_host_processor --host-processor "$HOST_PROCESSOR"
generate_go statechart_host_invoker --host-invoker "$HOST_INVOKER"
generate_go statechart_delayed_host_send --host-processor "$HOST_PROCESSOR"
# The one document that needs BOTH declarations (SCE Accepted Subset §2.15).
generate_go statechart_static_host_params \
    --host-processor "$HOST_PROCESSOR" --host-invoker "$HOST_INVOKER"
# No host declaration: the build names the Mesh request's type itself.
generate_go statechart_static_mesh_request
generate_go statechart_static_host_invoke --host-invoker "$HOST_INVOKER"
generate_go statechart_static_delayed_host_send --host-processor "$HOST_PROCESSOR"
generate_go statechart_host_invoker_optional --host-invoker "$HOST_INVOKER"
