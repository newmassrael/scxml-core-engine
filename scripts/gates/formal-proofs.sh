#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-proofs.yml
#
# The proofs of the host core under Verus, as docs/formal/proofs.json declares them
# (docs/adr/0115, D13): each file verified whole with nothing taken on trust, and each
# change of it that breaks what a user relies on refused. The judge's own checks
# (scripts/formal/proofs.py --self-test) run first, since a judge that regressed
# answers confidently and wrongly.
#
# Needs rustup and the Rust release the pinned Verus runs on; Verus is fetched once and
# checked against the digest the manifest pins. Without either the gate could not run,
# which is exit 3 and no verdict about the code.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 scripts/formal/proofs.py --self-test \
    || sce_gate_fail "the judge of the proofs is wrong (scripts/formal/proofs.py --self-test)"

status=0
python3 scripts/formal/proofs.py || status=$?
case "$status" in
    0) ;;
    3) sce_gate_cannot_run "no Verus, or not the Rust release it runs on" ;;
    *) sce_gate_fail "a proof is not as docs/formal/proofs.json declares it" ;;
esac
