#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Committed-tree drift + sourcemap suites, run serially (mirror of
# drift-verify.yml).
#
# `workspace-tests` runs with `--features cli`, so these two are already in
# its set. They are re-run here for one property that gate cannot give them:
# `--test-threads=1`. Both invoke the freshly built binary against the real
# committed tree, and drift-verify.yml runs them serially for that reason —
# a parallel run has two verify steps reading and rebuilding the same tree.
# The duplicate cost is seconds; the alternative is a drift gate that passes
# or fails by scheduling.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo test -p sce-build --features cli --test b9_drift_detection -- --test-threads=1 \
    || sce_gate_fail "b9_drift_detection — committed-tree drift"
cargo test -p sce-build --features cli --test sourcemap_addr2sce \
    || sce_gate_fail "sourcemap_addr2sce — traceability drift"
