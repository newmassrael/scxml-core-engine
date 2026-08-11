#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: sce-forge-codec-clippy.yml
#
# Clippy over the SCE-generated Rust codecs, with `alloc` on — the consumer's
# build configuration, which exercises the owned projection as well as the
# borrowed one.
#
# The workspace clippy gate cannot reach these. `cargo metadata` lists eight
# packages and no codec crate among them: the generated codecs are committed
# golden text under `tests/forge/expected/`, so `cargo clippy --workspace`
# lints the generator and never its output. That is the rustc-green /
# consumer-clippy-red gap, and until this gate existed the only thing standing
# in it was a CI workflow — which a developer sees after the push that
# introduced the regression, not before.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

./scripts/check_clippy_codec.sh \
    || sce_gate_fail "generated codecs are not clippy-clean with alloc on — fix the codec template, not the golden"
