#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Master regeneration for every committed §synth-6.2.6 generated tree
# tracked by the SCE workspace. Runs in three groups:
#
#   1. W3C IRP committed trees (Rust + Kotlin)
#   2. Integration committed trees (Rust + Kotlin + Go) via the
#      new `generate-integration` CLI surface (RFC
#      committed-tree refresh policy).
#   3. Forge round-trip Go codec tree.
#
# Build-time backends (cpp / c11 / Python) are intentionally absent
# — their generated trees materialise at CMake / CI time without a
# committed §synth-6.2.6 header to refresh.
#
# Use when a `tools/codegen/templates/` edit or a `Cargo.lock` bump
# invalidates the embedded `template-hash` on every committed tree
# (template-hash covers the whole template tree). Running this script + `git add -A` produces
# a single coherent commit that re-greens the drift-verify gate
# across every tracked context in one shot.
#
# Usage (from repo root):
#   scripts/regen_all_committed_trees.sh

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Pin the §synth-6.2.6 `generated-at` stamp (reproducible-builds
# convention, honoured by `forge::drift::now_utc_seconds`). Without it the
# stamp is wall-clock, so every regeneration rewrites all ~1100 committed
# files whether or not anything semantic moved — churn that trains
# reviewers to skim exactly the diffs a drift header exists to make
# visible, and that makes "regenerate and expect no diff" unexpressible as
# a gate. The stamp feeds neither hash, so pinning it costs no provenance.
# `committed_trees_carry_a_pinned_generated_at` fails if a regeneration
# lands without it.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

echo "==> W3C committed trees"
"$CODEGEN" generate-w3c -l rust
"$CODEGEN" generate-w3c -l kotlin
# The Go and Python W3C trees are .gitignored, so they carry no committed
# template-hash to re-green — but they are still real trees on a developer's
# disk, and leaving them out of the master refresh let them keep serving
# citations from templates that had already been corrected. A stale gitignored
# artifact is invisible to the drift gate and visible to every other check that
# walks the filesystem — and to the test suite: dropping a runtime symbol a
# template no longer emits left the stale Python tree importing it, failing
# collection for all 202 W3C modules at once.
"$CODEGEN" generate-w3c -l go
"$CODEGEN" generate-w3c -l python

echo "==> Integration trees (Rust / Kotlin / Go committed; Python gitignored)"
"$CODEGEN" generate-integration -l rust
"$CODEGEN" generate-integration -l kotlin
"$CODEGEN" generate-integration -l go
"$CODEGEN" generate-integration -l python

echo "==> Forge round-trip Go codec tree"
backends/go/forge-runtime/round_trip/generate.sh

# EventSchema native-lowering gates (NL→IR C1 Path A). Per-backend committed
# trees driven by their own regen scripts (not the `generate-integration`
# fan-out): each gate reuses the canonical fixture directly so its receive-side
# schema sibling resolves, and lives next to its own backend's test harness.
# Every backend (Rust, Go, Kotlin, Python; cpp/c11 gates run at CMake/CI time)
# now lowers the typed `_event.data` guard to a script-engine-free comparison.
echo "==> EventSchema native-lowering Rust tree"
scripts/regen_event_schema_native.sh
echo "==> EventSchema native-lowering Go tree"
scripts/regen_event_schema_native_go.sh
echo "==> EventSchema native-lowering Kotlin tree"
scripts/regen_event_schema_native_kotlin.sh
echo "==> EventSchema native-lowering Python tree"
scripts/regen_event_schema_native_python.sh

# W3C SCXML G.7 `<sce:action>` native host-trait dispatch gate. Rust-only
# committed tree (the other backends do not yet lower `<sce:action>`), driven
# by its own regen script like the EventSchema gates above. Part of the crate
# so it is really type-checked, so its §6.2.6 header must refresh with every
# template edit alongside the rest.
echo "==> Native-action host-trait Rust tree"
scripts/regen_native_action.sh

# The committed Rust trees are generator output *as rustfmt leaves it*, not
# as the emitter writes it. `backends/rust/tests` is a workspace member, so
# `cargo fmt --all` reformats it and `fmt-check.yml` requires that state —
# but nothing in the emitter produces it. Without this step the script's
# stated aim, "regenerate and expect no diff", is false for the W3C Rust
# tree: a plain regeneration leaves ~450 files differing from HEAD by
# whitespace alone. That gap used to be closed by accident, at commit time,
# by the pre-commit `cargo fmt --all -- --check` gate refusing the push
# until the developer ran the formatter — which meant the documented
# procedure did not reproduce the committed state, and a reviewer could not
# tell whitespace churn from a real emitter change.
#
# Scoped to the crate holding the committed generated Rust rather than
# `--all`, so the step says what it is for.
echo "==> Formatting the committed Rust trees (rustfmt is part of their committed form)"
cargo fmt -p sce-rust-tests

# The forge conformance goldens are generated from the same templates as
# everything above, and this script did not know about them.
#
# Measured: an `#include` added to `state_machine.jinja2` left
# `tests/forge/expected/inline_mixed_sm.{h,inl}` stale. The full regeneration
# above rewrote 1593 files and not those two; `regen-reproduces` passed them
# as well, because neither reads that tree. The drift surfaced four gates
# later in `workspace-tests` — twenty minutes into a push, on a template edit
# made hours earlier.
#
# Regenerating them here is what makes this script's stated aim true for the
# whole tree rather than for the parts it happened to enumerate. The
# conformance test is the generator: with UPDATE_GOLDEN set it writes the
# expectation instead of asserting against it.
echo "==> Regenerating the forge conformance goldens"
UPDATE_GOLDEN=1 cargo test -p sce-build --features cli --test forge_conformance --quiet

echo "All committed §6.2.6 trees regenerated."
