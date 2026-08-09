#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Python arm of the forge conformance suite (forge-conformance.yml).
#
# No build step, so this is the cheapest of the four language arms.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

( cd backends/python/forge-runtime && PYTHONPATH=tests python3 -m unittest \
    tests.test_numerical_conformance tests.test_default_round_trip ) \
    || sce_gate_fail "Python forge conformance"
