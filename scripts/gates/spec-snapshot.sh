#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: spec-snapshot-drift.yml
#
# Spec snapshot integrity + verifies-catalog drift (mirror of
# spec-snapshot-drift.yml).
#
# The gate whose absence turned main red on 2026-08-04. A `SCE-VERIFIES:`
# marker inside a test file is the catalog's source of truth, so adding a
# marked test without regenerating the catalog is drift — and
# `ledger-citations` cannot see it, because that gate runs `mnemosyne-cli`
# while this check is a separate python generator wired only into the CI
# workflow. A hook that mirrors CI gate-by-gate has to mirror this one too;
# the push it blocks otherwise succeeds locally and fails on the runner.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Snapshot integrity is a sha256 comparison against the pinned provenance —
# no network, no build. `--mode upstream` is the one that fetches, and it
# stays CI-only (see CI_ONLY in sce-build/tests/hook_ci_parity.rs).
python3 tools/mnemosyne-adoption/check_spec_drift.py --mode integrity \
    || sce_gate_fail "spec snapshot does not match its provenance"

# Two catalogs, two scripts, different files. The hook ran only the mesh one
# for long enough that the asymmetry read as intentional.
python3 tools/mnemosyne-adoption/gen_verifies_catalog.py --check \
    || sce_gate_fail "verifies-catalog stale — regenerate with tools/mnemosyne-adoption/gen_verifies_catalog.py"
python3 tools/mnemosyne-adoption/gen_mesh_verifies_catalog.py --check \
    || sce_gate_fail "mesh verifies-catalog stale — regenerate with tools/mnemosyne-adoption/gen_mesh_verifies_catalog.py"

# These tools drive the ledger validation gate. If they regress, that gate
# answers confidently and wrongly, so their own tests belong on the same
# side of the push.
python3 -m unittest discover -s tools/mnemosyne-adoption/tests \
    || sce_gate_fail "mnemosyne-adoption tooling regression"
