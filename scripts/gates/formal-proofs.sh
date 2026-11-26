#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-proofs.yml
#
# The proofs of the host core under Verus, as docs/formal/proofs.json declares them
# (docs/adr/0115, D13): each file verified whole with nothing taken on trust, and each
# change of it that breaks what a user relies on refused.
#
# The judge is `sce-codegen formal proofs` (docs/adr/0146), the command a consumer
# runs on its own manifest. Its own checks are its unit tests, which the Rust lanes run.
#
# Needs the pinned Verus and the Rust release it runs on, which bx.toml declares and no
# gate installs; anywhere but a build machine they are put at the same places by
# `sce-codegen formal install --tool verus` and `scripts/install_rust.sh --declared`.
# Without either the judge exits 15 (`formal/tool-unavailable`), no verdict about the code.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
status=0
target/debug/sce-codegen formal proofs --manifest docs/formal/proofs.json || status=$?
case "$status" in
    0) ;;
    15) sce_gate_cannot_run "no Verus, or not the Rust release it runs on" ;;
    *) sce_gate_fail "a proof is not as docs/formal/proofs.json declares it" ;;
esac
