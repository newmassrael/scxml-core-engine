#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# Rust here; Go, Kotlin and Python each from a script of their own
# (`regen_host_processor_go.sh`, `regen_host_processor_kotlin.sh`,
# `regen_host_processor_python.sh`). C++ and C11 drive the same fixture from
# their own build-time generation (`tests/CMakeLists.txt` and
# `backends/c/tests/CMakeLists.txt`), so they need no tree here. A backend
# without a registry is refused the declaration by name
# (`reject_host_processors_in_unsupported_lang`) rather than handed a dispatch
# nothing can service. That refusal is what keeps a partial roster honest
# instead of a coverage gap: a declaration cannot silently compile on a backend
# that would drop it. Called directly from regen_all_committed_trees.sh rather
# than through the `generate-integration` fan-out, which has no per-stem flags —
# the same reason regen_native_action.sh is called directly.
#
# This script wrote the Go trees too until 2026-11-07. They moved to
# `regen_host_processor_go.sh` because the regeneration runs as one bx action
# per language tree (bx.toml, the `regen-*` actions): a script that writes two
# trees belongs to no one action, and an action is refused when it writes
# outside the outputs it declares.
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

# The typed-parameter fixture: a `datamodel="sce-static"` machine, so a
# `<param expr>` is a typed expression over the machine's own fields and not
# script text. A fourth document because it is the one that needs BOTH
# declarations — it sends to the host and invokes it — and the other three
# each hold one axis.
STATIC_PARAMS_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml"

# The saving fixture: a `datamodel="sce-static"` machine whose host-run
# `<invoke>` a saved state carries (SCE Accepted Subset §2.15). Invoker
# declaration only — it sends nothing — and a document of its own because what
# it measures is what a restore does with a request, not what a request holds.
STATIC_INVOKE_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_static_host_invoke.scxml"

# The other saving fixture: a `datamodel="sce-static"` machine whose delayed
# host-served `<send>` a saved state carries. Processor declaration only — it
# invokes nothing — and a document of its own because the three kinds of waiting
# act (a send to itself, a host invocation, a host send) are separate things a
# restore must each give back.
STATIC_DELAYED_SEND_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_static_delayed_host_send.scxml"

# The Mesh request: a `datamodel="sce-static"` machine whose `<invoke
# type="sce:mesh-rpc">` names its peer and writes its params from the machine's
# fields (docs/adr/0005, decisions 5 and 7). NO host declaration: the build
# lowers the request to a host-served invoke of its own type, so the router the
# host registers through the mesh-rpc door is what it starts. A document of its
# own because the type is the build's and not the host's.
STATIC_MESH_REQUEST_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml"

# The optional-result fixture (docs/adr/0055, step 10): a `datamodel="sce-static"`
# machine whose host-run `<invoke>` completes with a `sce:result` record holding
# values that may be absent, which the host hands in through the generated typed
# interface and the engine delivers as JSON with `null` for an absent one.
# Invoker declaration only, and a document of its own because the invoke the
# first fixture types holds scalars that are always present.
OPTIONAL_RESULT_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml"

# The inbox probe (docs/adr/0084): the smallest machine with a host-run
# `<invoke>` whose every taken event can be read back in order, which
# backends/rust/tests/tests/what_another_thread_posts_enters_through_the_inbox.rs
# drives while other threads post to its inbox. It sits beside the generated C++
# engine's driver of the same case, whose tree tests/CMakeLists.txt generates at
# build time, so only the Rust tree is committed from it here.
INBOX_PROBE_FIXTURE="tests/integration/inbox_probe.scxml"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"
"$CODEGEN" generate "$INVOKER_FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"
"$CODEGEN" generate "$DELAYED_FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"
"$CODEGEN" generate "$STATIC_PARAMS_FIXTURE" -l rust -o "$TMP/" \
    --host-processor "$HOST_PROCESSOR" --host-invoker "$HOST_INVOKER"
"$CODEGEN" generate "$STATIC_INVOKE_FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"
"$CODEGEN" generate "$STATIC_DELAYED_SEND_FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"
"$CODEGEN" generate "$STATIC_MESH_REQUEST_FIXTURE" -l rust -o "$TMP/"
"$CODEGEN" generate "$OPTIONAL_RESULT_FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"
"$CODEGEN" generate "$INBOX_PROBE_FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
cp "$TMP"/*_sm.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_host_processor.sh)"
    echo ""
    echo "mod inbox_probe_sm;"
    echo "mod statechart_delayed_host_send_sm;"
    echo "mod statechart_host_invoker_optional_sm;"
    echo "mod statechart_host_invoker_sm;"
    echo "mod statechart_host_processor_sm;"
    echo "mod statechart_static_delayed_host_send_sm;"
    echo "mod statechart_static_host_invoke_sm;"
    echo "mod statechart_static_host_params_sm;"
    echo "mod statechart_static_mesh_request_sm;"
    echo "pub use inbox_probe_sm::*;"
    echo "pub use statechart_delayed_host_send_sm::*;"
    echo "pub use statechart_host_invoker_optional_sm::*;"
    echo "pub use statechart_host_invoker_sm::*;"
    echo "pub use statechart_host_processor_sm::*;"
    echo "pub use statechart_static_delayed_host_send_sm::*;"
    echo "pub use statechart_static_host_invoke_sm::*;"
    echo "pub use statechart_static_host_params_sm::*;"
    echo "pub use statechart_static_mesh_request_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from"
echo "  $FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "  $INVOKER_FIXTURE (--host-invoker $HOST_INVOKER)"
echo "  $DELAYED_FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "  $OPTIONAL_RESULT_FIXTURE (--host-invoker $HOST_INVOKER)"
echo "  $INBOX_PROBE_FIXTURE (--host-invoker $HOST_INVOKER)"
