#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: none
#
# Build the generator every downstream gate executes.
#
# A prerequisite rather than a gate in its own right: `nostd-mcu`,
# `forge-go`, `forge-cpp` and `example-codegen` all run
# `target/debug/sce-codegen`, and declare it through `deps` in the registry
# so selecting any of them pulls this in.
#
# Debug profile. The generator's cost is process start-up and I/O, not
# optimisation — measured at 2.75s vs 2.89s over forty runs of the largest
# fixture, with the two profiles producing byte-identical output across all
# 1588 committed generated files. Building it in release compiled the entire
# dependency tree a second time, sharing nothing with the clippy or workspace
# test runs: 145.7s against 24.6s in a tree those gates had already warmed.
# `cargo test --release` in `forge-rust` is a different thing and stays —
# that one pins optimised floating-point output, which is what ships.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo build --bin sce-codegen --features cli -p sce-build \
    || sce_gate_fail "sce-codegen build"
