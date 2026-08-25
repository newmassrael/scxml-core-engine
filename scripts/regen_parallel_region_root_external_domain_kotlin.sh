#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/parallel_region_root_external_domain/
# from tests/integration/parallel_region_root_external_domain.scxml.
#
# The Kotlin half of the transition-domain witness. The Rust half is
# `scripts/regen_parallel_region_root_external_domain.sh`, the Go half
# `..._go.sh`, the Python half `..._python.sh`, and the two C++ drivers plus the
# C11 one build from the same document through CMake.
#
# This channel is the LAST of the six to get a witness, and it is the one that
# needed no repair to pass: Kotlin was already the only engine filtering
# `findLCCA`'s candidates with `isCompoundStateOrScxmlElement`, which is how the
# divergence was found at all — it answered `session.lost` differently from
# every sibling on `examples/ai_loop/ai_loop.scxml`. A channel that was right
# all along is exactly the one whose witness is easiest to leave unwritten, and
# leaving it unwritten is what would let a later edit regress the one engine
# that had the rule.
#
# The source is NOT under `integration_resources/`, and that is deliberate. A
# stem there is a seven-channel contract that `integration_stem_registration.rs`
# enforces, and this document has six drivers rather than seven — there is no
# mesh channel for it. The document lives beside its first driver instead.
#
# No `--input-root` and no tmp staging of the fixture: this document declares no
# `<invoke>`, so codegen emits exactly one `*Sm.kt` and sets the drift header's
# source path from the canonical input path directly. `--kotlin-package-prefix`
# is still needed — it flips the emitted `package` header off
# `com.sce.generated.<stem>` so the driver beside it can import the machine.
#
# Usage (from repo root):
#   scripts/regen_parallel_region_root_external_domain_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable. `scripts/lib/sce_codegen.sh` pins
# `generated-at` to SOURCE_DATE_EPOCH=0, which is what
# `committed_trees_carry_a_pinned_generated_at` requires of a committed tree.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

FIXTURE="tests/integration/parallel_region_root_external_domain.scxml"
GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/parallel_region_root_external_domain"
PACKAGE_PREFIX="com.sce.integration"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

# Clear stale `*Sm.kt` first so a renamed artefact does not leave the previous
# one orphaned beside the new one.
mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete

cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
