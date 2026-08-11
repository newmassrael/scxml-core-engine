#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Go arm of the forge conformance suite (forge-conformance.yml).
#
# generate.sh already runs `go build ./conformance/...` as a smoke check, so
# a codegen drift that produces uncompilable Go (the package-alias bug this
# gate was born to catch) surfaces right here.
#
# The regenerate is the expensive half; running the suite it just produced
# costs milliseconds. Building an artifact and then not looking at the
# result is how forge-conformance.yml came to be mirrored in name only.
#
# ./round_trip/ is the committed tree, orthogonal to the header-hash drift
# gate: that one checks parity, this checks the tree still functions.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

( cd backends/go/forge-runtime/conformance && bash generate.sh ) \
    || sce_gate_fail "Go forge conformance generate"
( cd backends/go/forge-runtime && go test ./conformance/ -count=1 ) \
    || sce_gate_fail "Go forge conformance"
( cd backends/go/forge-runtime && go test ./round_trip/ -count=1 ) \
    || sce_gate_fail "Go forge round-trip"
