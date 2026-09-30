#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Python arm of the forge conformance suite (forge-conformance.yml).
#
# No build step, so this is the cheapest of the four language arms.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The generated cases in numerical_reference.json are the model's output
# (tests/forge/conformance/gen_cases.py, E11): a file the generator would not
# write is either hand-edited in a generated line or stale against the model,
# and every arm below would then be held to cases nobody can reproduce. Run
# here because this arm has no build step and Python is all it needs.
python3 tests/forge/conformance/gen_cases.py --check \
    || sce_gate_fail "generated conformance cases are stale — run tests/forge/conformance/gen_cases.py"

( cd backends/python/forge-runtime && PYTHONPATH=tests python3 -m unittest \
    tests.test_numerical_conformance tests.test_default_round_trip ) \
    || sce_gate_fail "Python forge conformance"
