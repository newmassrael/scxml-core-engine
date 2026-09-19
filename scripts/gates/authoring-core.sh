#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: authoring-core.yml
#
# The authoring core turns a prose specification into a document, and it claims
# to know nothing about what the specification is about. That claim is the
# whole point of the package: the moment it knows one subject matter, it stops
# being usable for the next, and the way it stops being true is that somebody
# adds one helpful special case for the subject in front of them.
#
# So the claim is a test rather than a sentence in a README, and the test runs
# here. Two unlike guards, because a word list is only as wide as somebody's
# imagination: no source may spell a subject matter, and no source may name a
# location of its own (a hard-coded path is how a tool acquires a home).
#
# The refusal suite is on the same side of the push for a different reason. A
# guard that has never fired is a guard nobody has measured, and every refusal
# the core can make is built and asserted there -- each paired with the nearest
# situation that must NOT be refused, so a check that refuses everything fails.
#
# Python stdlib plus PyYAML and jsonschema. No engine toolchain.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 -c 'import yaml, jsonschema' \
    || sce_gate_fail "authoring core needs PyYAML and jsonschema"

( cd tools/authoring && PYTHONPATH=. python3 -m unittest discover -s tests -t . ) \
    || sce_gate_fail "authoring core regression (domain leak, or a refusal that no longer fires)"
