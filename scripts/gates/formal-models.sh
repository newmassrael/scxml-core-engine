#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-models.yml
#
# The TLA+ models of docs/formal under TLC, as docs/formal/models.json declares
# them (docs/adr/0115, D5): every configuration explored whole, its count of
# distinct states as recorded, and every part of a protocol taken out in turn,
# where TLC must find the violation the manifest names.
#
# It was a script a person ran: no workflow, gate or bx declaration called it, so
# a model could go stale with no verdict. The judge's own checks
# (scripts/formal/models.py --self-test) run first, since a judge that regressed
# answers confidently and wrongly.
#
# Needs Java 11 or later; TLC is fetched once and checked against the digest the
# manifest pins. Without either the gate could not run, which is exit 3 and no
# verdict about the models.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 scripts/formal/models.py --self-test \
    || sce_gate_fail "the judge of the models is wrong (scripts/formal/models.py --self-test)"
# What `pre-push` judges the commits of a push by (docs/adr/0115, D11); its tests run here.
python3 tools/git-hooks/spec_isolation.py --self-test \
    || sce_gate_fail "the judge of what changes a check is wrong (tools/git-hooks/spec_isolation.py --self-test)"

status=0
python3 scripts/formal/models.py || status=$?
case "$status" in
    0) ;;
    3) sce_gate_cannot_run "no Java or no TLC for the models" ;;
    *) sce_gate_fail "a model is not as docs/formal/models.json declares it" ;;
esac
