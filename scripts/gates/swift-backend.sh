#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift backend's own lane: the three SwiftPM packages under `backends/swift/`
# built and tested with the toolchain the workflow pins.
#
#   runtime          the statechart runtime (`SceRuntime`)
#   forge-runtime    the checked arithmetic and bounded values the generated code calls
#   tests            the machines the generator writes, replayed against the shared
#                    scenarios of `sce-build/tests/fixtures/static_datamodel/scenarios/`
#
# Until this gate existed nothing a hosted runner ran compiled a line of Swift: the other
# backends' suites are reached through their W3C and Forge lanes, and Swift has neither
# lane yet (its `ACCEPTED` rows in `a_backend_holds_what_the_other_backends_hold` say so).
# `regen-reproduces` judges that the committed Swift machines are what the generator writes;
# this judges that what the generator writes COMPILES and BEHAVES.
#
# `scripts/test_swift_backend.sh` regenerates the machines before it compiles them, so what
# runs is what the templates write today. A tree that drifted from the generator is
# `regen-reproduces`' finding and not this gate's, and the two are kept apart on purpose: a
# gate that regenerated and compared would be a second `regen-reproduces` with a Swift
# toolchain's cost.
#
# The verdict is read from the output as well as from the exit status. A package whose test
# target was not discovered builds, runs nothing and exits 0, which is the shape of a lane
# that was never testing. Each package must report a run of at least one test.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

WORKFLOW="$SCE_REPO_ROOT/.github/workflows/swift-backend.yml"

# The toolchain is pinned twice, in `bx.toml` (what `provision` installs on the build machines)
# and in the workflow (what the hosted runner installs), and nothing derives one from the other.
# The gate reads the workflow's version, so a build machine on the release bx.toml names and a
# runner on another would each pass and disagree about which compiler the packages are verified
# on. Held equal here, where both files are inputs of the lane's own `paths:` filter: the
# release, and the digest of the Ubuntu 24.04 tarball the runner checks.
bx_version="$(awk '/^\[/ { in_swift = ($0 == "[tools.swift]"); next }
    in_swift && /^version = / { gsub(/[" ]/, "", $3); print $3; exit }' bx.toml)"
bx_digest="$(grep -o 'ubuntu24\.04) sum=[0-9a-f]\{64\}' bx.toml | head -1 | sed 's/.*sum=//')"
pin_version="$(sed -n "s/^[[:space:]]*SWIFT_VERSION:[[:space:]]*['\"]\{0,1\}\([0-9][0-9.]*\).*/\1/p" "$WORKFLOW" | head -1)"
pin_digest="$(sed -n 's/^[[:space:]]*SWIFT_SHA256_UBUNTU2404:[[:space:]]*\([0-9a-f]\{64\}\).*/\1/p' "$WORKFLOW" | head -1)"
[ -n "$bx_version" ] && [ -n "$bx_digest" ] \
    || sce_gate_fail "bx.toml carries no \`[tools.swift]\` version or no Ubuntu 24.04 digest in its install command: the pin this gate compares the workflow's against is unreadable"
[ -n "$pin_version" ] && [ -n "$pin_digest" ] \
    || sce_gate_fail "swift-backend.yml carries no SWIFT_VERSION or no SWIFT_SHA256_UBUNTU2404"
[ "$pin_version" = "$bx_version" ] \
    || sce_gate_fail "swift-backend.yml installs Swift $pin_version and bx.toml installs $bx_version on the build machines: the packages would be verified on a compiler the build machines do not have"
[ "$pin_digest" = "$bx_digest" ] \
    || sce_gate_fail "swift-backend.yml checks a different Ubuntu 24.04 tarball digest than bx.toml does for Swift $bx_version"
sce_gate_step "Swift $pin_version and its Ubuntu 24.04 digest are the same in bx.toml and the workflow"

sce_gate_require_swift "$WORKFLOW"

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
scripts/test_swift_backend.sh 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh exited $status (a Swift package failed to build or a test failed; the output above names it)"

# One `Test run with N tests` line per test target that ran: the runtime, the forge runtime,
# and the tests package's two targets.
runs="$(grep -cE 'Test run with [0-9]+ tests? ' "$LOG" || true)"
[ "$runs" -ge 3 ] \
    || sce_gate_fail "$runs Swift test runs reported, expected one per package (3 or more): a package built and ran nothing"
if grep -qE 'Test run with 0 tests ' "$LOG"; then
    sce_gate_fail "a Swift test run reported 0 tests: its test target was not discovered"
fi
sce_gate_step "$runs Swift test runs, each of at least one test"
