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
# Rust and Go, the two backends with a committed tree AND a host-processor
# registry. C++ and C11 drive the same fixture from their own build-time
# generation (`tests/CMakeLists.txt` and `backends/c/tests/CMakeLists.txt`),
# so they need no tree here. Kotlin and Python have no registry yet, and the
# generator refuses the declaration for them by name
# (`reject_host_processors_in_unsupported_lang`) rather than emitting a
# dispatch nothing can service. That refusal is what keeps a partial roster
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

# The invoke-side fixture. A separate document because the two surfaces are
# separate contracts — delivering an event is not the same capability as
# running an invoked process — and a single fixture would make one gate
# stand for both. Declared with the invoker flag, not the processor one.
INVOKER_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml"
HOST_INVOKER="x-sce-host"

# The delay-side fixture. A third document rather than a `delay` added to the
# one above, because that one is driven with `step` and this one can only be
# driven with `tick`: folding them together would put a clock under every
# assertion the first one makes, and a fixture measuring two things cannot say
# which of them broke. W3C SCXML 6.2.4 puts the wait before the dispatch
# whatever processor the send named, and W3C SCXML 6.3 lets a `<cancel>` drop
# one that is still waiting — two observations of the single fact that a
# delayed host-served send is in the delayed-send queue like any other.
DELAYED_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"
"$CODEGEN" generate "$INVOKER_FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"
"$CODEGEN" generate "$DELAYED_FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*_sm.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_host_processor.sh)"
    echo ""
    echo "mod statechart_delayed_host_send_sm;"
    echo "mod statechart_host_invoker_sm;"
    echo "mod statechart_host_processor_sm;"
    echo "pub use statechart_delayed_host_send_sm::*;"
    echo "pub use statechart_host_invoker_sm::*;"
    echo "pub use statechart_host_processor_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

# The Go half. Only the processor fixture: Go has a `<send>` registry and no
# invoker one, so generating the invoker document here would meet the refusal
# that is still correct for it.
#
# The `// From:` rewrite is the same one every `regen_*_go.sh` does — the
# tracked artefact has to point at the canonical fixture rather than at the
# temporary directory this run happened to use, or the tree stops reproducing
# on another machine.
# Named for the document's own stem rather than for the axis, unlike the Rust
# tree above: a Go package name comes from the emitted file, so a directory
# named anything else puts two package names in one directory and the module
# stops building.
GO_GENERATED_DIR="backends/go/tests/integration/statechart_host_processor"
GO_TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$GO_TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$GO_TMP/" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GO_GENERATED_DIR"
find "$GO_GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete
for src in "$GO_TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${GO_TMP}/|// From: $(dirname "$FIXTURE")/|g" "$src"
done
cp "$GO_TMP"/*_sm.go "$GO_GENERATED_DIR/"

# The delayed document gets its own Go directory for the reason stated above:
# a Go package name comes from the emitted file, so two stems in one directory
# is two package names in one directory.
GO_DELAYED_DIR="backends/go/tests/integration/statechart_delayed_host_send"
GO_DELAYED_TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$GO_TMP" "$GO_DELAYED_TMP"' EXIT

"$CODEGEN" generate "$DELAYED_FIXTURE" -l go -o "$GO_DELAYED_TMP/" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GO_DELAYED_DIR"
find "$GO_DELAYED_DIR" -maxdepth 1 -name '*_sm.go' -delete
for src in "$GO_DELAYED_TMP"/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${GO_DELAYED_TMP}/|// From: $(dirname "$DELAYED_FIXTURE")/|g" "$src"
done
cp "$GO_DELAYED_TMP"/*_sm.go "$GO_DELAYED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from"
echo "  $FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "  $INVOKER_FIXTURE (--host-invoker $HOST_INVOKER)"
echo "  $DELAYED_FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "Regenerated: $GO_GENERATED_DIR/ from"
echo "  $FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "Regenerated: $GO_DELAYED_DIR/ from"
echo "  $DELAYED_FIXTURE (--host-processor $HOST_PROCESSOR)"
