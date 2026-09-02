#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: clippy-check.yml
#
# cargo clippy over the workspace (mirror of clippy-check.yml).
#
# clippy-check.yml delegates here rather than restating the command, so
# the lint set has one spelling for both callers. It runs on
# dtolnay/rust-toolchain@stable, and the lint set is version-sensitive: a
# stale local `stable` (behind the CI runner's fresh stable) passes
# locally and only breaks CI on a newly-stabilised lint — exactly the
# unnecessary_sort_by / collapsible_match drift that motivated this gate.
# Toolchain parity is enforced by the runner before any gate starts, so
# this sees the same lints CI will.
#
# `--features cli` is part of the target selection, not a flavour of it.
# `--all-targets` asks for every target the feature set ALLOWS, and cargo
# drops one whose `required-features` are unmet without building it and
# without reporting a skip. Measured 2026-09-02, on the commit that first
# declared those features: clippy saw 135 of sce-build's integration
# targets and none of the 53 that spawn `sce-codegen` — including 25 it
# had been linting the day before, which the declaration silently took
# away. `cli_feature_gating` is what fails if this flag goes missing
# again.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo clippy --workspace --all-targets --features cli -- -D warnings \
    || sce_gate_fail "cargo clippy --workspace --all-targets --features cli"
