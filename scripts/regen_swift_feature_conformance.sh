#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Write the machines of backends/swift/feature-conformance (docs/adr/0057 §10): the
# Swift machine of tests/product_line/session.scxml generated once for every build,
# copied into the target of each feature set (`Lifted<Set>`, compiled with that set's
# `.define`s), and the machine each set's variant generates alone (`Alone<Set>`).
#
# Neither is committed (`.gitignore`): the lane judges what the generator writes today.
#
# Usage (from repo root):
#   scripts/regen_swift_feature_conformance.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
DOCUMENT="tests/product_line/session.scxml"
SOURCES="backends/swift/feature-conformance/Sources"

# Each feature set: the name of its targets, and the features it turns on. The
# `.define`s of each are in the package's manifest.
FEATURE_SETS=("None:" "Unicast:session-unicast" "Accept:session-unicast,session-unicast-accept")

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# The sidecar the generation writes beside a machine is no source of the target.
"$CODEGEN" generate "$DOCUMENT" -l swift -o "$TMP/lifted/"
rm -rf "$SOURCES"
for set in "${FEATURE_SETS[@]}"; do
    name="${set%%:*}"
    features="${set#*:}"
    "$CODEGEN" generate "$DOCUMENT" -l swift -o "$TMP/alone-$name/" "--features=$features"
    mkdir -p "$SOURCES/Lifted$name" "$SOURCES/Alone$name"
    cp "$TMP/lifted/"*.swift "$SOURCES/Lifted$name/"
    cp "$TMP/alone-$name/"*.swift "$SOURCES/Alone$name/"
done

echo "Regenerated: the machine for every build and each feature set's own under $SOURCES/"
