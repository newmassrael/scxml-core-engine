#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/W3cMachines/ and the tests beside it
# (backends/swift/tests/Tests/W3cMachinesTests/Test<id>Tests.swift) from the W3C documents of
# `tests/w3c/conformance/fixtures.json` that the Swift backend lowers.
#
# This is `sce-codegen generate-w3c -l swift`, the route every other backend's W3C suite is written
# by, and nothing more: what a fixture's test asserts, which fixtures belong to the `simple` harness,
# how a document is rejected, how a child session is written beside its parent, and which refusals
# are documents not lowered yet are all the generator's, and are not said again here. The suite is
# not committed (as Go's is not), since a template edit would then rewrite a file per document, so
# the lane regenerates it before it runs it, which also makes the lane judge the generator in the
# working tree and not an artifact somebody produced earlier.
#
# Only the `simple` harness for now (run the machine to completion and assert it settles in its
# `pass` state, or its `final` state when it has no `pass`): `scheduled` needs a clock the test
# advances and `http` the echo endpoint. A fixture of another harness, and a document the backend
# refuses by name (`generate/unsupported-feature`), is listed under "Skipped" with its reason.
#
# What this script adds is the floor, which is a policy of the lane and not of the generator.
#
# Usage (from repo root):
#   scripts/regen_swift_w3c.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# A floor, not an equality: a construct Swift gains lowers more fixtures without touching this
# number, but a document the generator stopped lowering must not read as a smaller suite that
# passed. Raise it when a slice lands; measured 2026-10-31 at 62 of 157.
FLOOR=156

# Every generated file is cleared first, by what each file says it is, so a document no longer
# lowered does not leave its machine or its test behind, and neither does a file an earlier shape of
# this suite wrote. What is written by hand (`W3cSupport.swift`) is left alone.
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear --recursive backends/swift/tests/Sources/W3cMachines
sce_generated_tree_clear backends/swift/tests/Tests/W3cMachinesTests

SUMMARY="$(mktemp)"
trap 'rm -f "$SUMMARY"' EXIT
"$CODEGEN" generate-w3c -l swift > "$SUMMARY"
cat "$SUMMARY"

GENERATED="$(sed -n 's/^  Generated (total): *//p' "$SUMMARY")"
if [ -z "$GENERATED" ]; then
    echo "error: generate-w3c -l swift did not say how many fixtures it generated" >&2
    exit 1
fi
if [ "$GENERATED" -lt "$FLOOR" ]; then
    echo "error: the Swift backend lowers $GENERATED simple W3C fixture(s), under the floor of $FLOOR: a construct it lowered is refused again" >&2
    exit 1
fi
echo "Swift W3C (simple harness): $GENERATED fixtures lowered into backends/swift/tests/Sources/W3cMachines/"
