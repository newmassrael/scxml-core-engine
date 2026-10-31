#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: swift-backend.yml
#
# The Swift W3C arm: the W3C documents of the `simple` harness that the Swift backend lowers,
# written by `sce-codegen generate-w3c -l swift` and run against the engine, each settling in its
# `pass` state. It is the Swift twin of `w3c-go`, and `swift-backend` keeps judging the packages.
#
# The generated suite is not committed, so this gate writes it before it runs it, exactly as the
# lane does, and it judges the generator in the working tree and not an artifact somebody produced
# earlier. A document the backend refuses by name, and the fixtures of the harnesses it has not
# built (`scheduled`, `http`), are listed under "Skipped" in the generator's summary with the reason
# and are not tests: what this gate holds is that every fixture the generator DID write ran.
#
# The floor on how many the generator lowers is `scripts/regen_swift_w3c.sh`'s, and is not said
# again here.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The toolchain is the one `bx.toml` pins, which is also what the runner installs
# (`scripts/install_swift_toolchain.sh`).
sce_gate_require_swift

LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$LOG'"

# The status of the script, not of the `tee` it is piped to.
set +e
scripts/test_swift_backend.sh w3c 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] \
    || sce_gate_fail "scripts/test_swift_backend.sh w3c exited $status (the W3C suite failed to generate, to build or to pass; the output above names it)"

# The number the lane says it wrote, and the number the run says it ran. A suite whose tests were
# not discovered builds, runs nothing and exits 0, which is the shape of a lane that was never
# testing; and one that ran fewer tests than it wrote has dropped fixtures without saying so.
written="$(sed -n 's/^Swift W3C (simple harness): \([0-9][0-9]*\) fixtures lowered.*/\1/p' "$LOG" | tail -n 1)"
ran="$(grep -oE 'Test run with [0-9]+ tests?' "$LOG" | tail -n 1 | grep -oE '[0-9]+' || true)"
[ -n "$written" ] \
    || sce_gate_fail "the lane did not say how many W3C fixtures it wrote"
[ -n "$ran" ] \
    || sce_gate_fail "no Swift test run was reported: the W3C test target was not discovered"
[ "$ran" -eq "$written" ] \
    || sce_gate_fail "the generator wrote $written W3C fixture(s) and $ran test(s) ran: a fixture was written and not run, or run twice"
sce_gate_step "$ran W3C fixtures settled in their pass state"
