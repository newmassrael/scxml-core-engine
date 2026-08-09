#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Rust arm of the forge conformance suite (forge-conformance.yml).
#
# `--release` is not incidental: the numerical conformance vectors are
# compared against optimised floating-point output, which is what ships.
# `workspace-tests` runs the workspace in debug and cannot stand in for it.
# The separate profile costs a one-time release build; afterwards it is
# incremental — near-zero on a push that does not touch the crate, which is
# most of them.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo test --release -p sce-forge-runtime --features alloc --test numerical_conformance \
    || sce_gate_fail "Rust forge conformance"
