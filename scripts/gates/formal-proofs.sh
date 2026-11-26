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
# Runs in the host image. The Rust release Verus runs on is the machine's, under
# ~/.local/rust, which scripts/gate mounts into the image (bx.toml declares it for the
# build machines; a lane installs it with `scripts/install_rust.sh --declared`). Verus is
# fetched by `sce-codegen formal install --tool verus` below and admitted by its digest,
# or found in the image's home; the judge itself never fetches. Without either the judge
# exits 15 (`formal/tool-unavailable`), no verdict about the code.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
install=0
target/debug/sce-codegen formal install --tool verus || install=$?
case "$install" in
    0) ;;
    15) sce_gate_cannot_run "the pinned Verus could not be fetched or admitted" ;;
    *) sce_gate_fail "sce-codegen formal install --tool verus ended $install" ;;
esac

status=0
target/debug/sce-codegen formal proofs --manifest docs/formal/proofs.json || status=$?
case "$status" in
    0) ;;
    15) sce_gate_cannot_run "no Verus, or not the Rust release it runs on" ;;
    *) sce_gate_fail "a proof is not as docs/formal/proofs.json declares it" ;;
esac
