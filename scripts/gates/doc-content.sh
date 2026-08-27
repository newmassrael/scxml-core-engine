#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: doc-content-gate.yml
#
# The sce-build library suite, run because a compiled-in INPUT changed
# rather than because its Rust did.
#
# `sce-build/src/` `include_str!`s documents, schemas, headers and Lua into
# the test binary, so editing one changes what the suite asserts without
# touching a `.rs`. `workspace-tests` runs the same assertions but its
# filter is narrow on purpose — it is 151s, half the push budget — so the
# wide filter lives on this lane instead and the expensive one keeps the
# filter it has.
#
# `--lib` rather than a list of test names: every input this lane exists
# for is read from `sce-build/src/`, so the library suite is exactly the
# set of assertions that see them. A named list would be a hand-kept list
# of the same shape as the one `include-str-coverage` exists to retire.
# `--features cli` because a target whose `required-features` are unmet is
# excluded silently, without building and without reporting a skip.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo test -p sce-build --features cli --lib \
    || sce_gate_fail "sce-build library suite (compiled-in inputs)"
