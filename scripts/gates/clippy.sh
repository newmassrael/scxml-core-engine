#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: clippy-check.yml
#
# cargo clippy over the workspace (mirror of clippy-check.yml).
#
# clippy-check.yml runs `cargo clippy --workspace --all-targets -- -D
# warnings` on dtolnay/rust-toolchain@stable. The lint set is
# version-sensitive: a stale local `stable` (behind the CI runner's fresh
# stable) passes locally and only breaks CI on a newly-stabilised lint —
# exactly the unnecessary_sort_by / collapsible_match drift that motivated
# this gate. Toolchain parity is enforced by the runner before any gate
# starts, so this sees the same lints CI will.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo clippy --workspace --all-targets -- -D warnings \
    || sce_gate_fail "cargo clippy"
