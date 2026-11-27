#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-traces.yml
#
# The recorded runs of the code under TLC, as docs/formal/traces.json declares
# them (docs/adr/0115, D4 and D8): every run a lane's trace port wrote accepted
# as a behaviour of the model it names, and every altered run, one field of one
# step given another value, refused at the very step that was altered.
#
# The judge is `sce-codegen formal traces` (docs/adr/0146), the command a
# consumer runs on its own manifest. Its own checks are its unit tests, which
# the Rust lanes run; that a recorded run is the run the code makes now is held
# by the lane that records it (the Rust lane's
# `a_run_is_a_behaviour_of_its_model`).
#
# Runs in the host image. The pinned Java runtime and TLC are fetched by
# `sce-codegen formal install` below and admitted by their digests, or found in
# the image's home; so is the pinned Apalache, whose standard module `Apalache`
# TLC is given for a model beside the traces that extends it. The judge itself
# never fetches, and never runs the JDK the image carries for the Kotlin lanes.
# Without a pinned tool or its runtime the judge exits 15
# (`formal/tool-unavailable`), which is "could not run" and no verdict about
# the runs.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
for tool in java tlc apalache; do
    install=0
    target/debug/sce-codegen formal install --tool "$tool" || install=$?
    case "$install" in
        0) ;;
        15) sce_gate_cannot_run "the pinned $tool could not be fetched or admitted" ;;
        *) sce_gate_fail "sce-codegen formal install --tool $tool ended $install" ;;
    esac
done

# The processors this unit was given (`sce_build_jobs`: a build machine's
# grant, `BX_GRANT_CORES`, before anything the machine reads of itself), each
# run of TLC given them one after another, never TLC's `auto`
# (sce-build/src/formal/tlc.rs says why).
status=0
target/debug/sce-codegen formal traces --workers "$(sce_build_jobs)" \
    --manifest docs/formal/traces.json || status=$?
case "$status" in
    0) ;;
    15) sce_gate_cannot_run "not the pinned Java runtime, TLC or Apalache: sce-codegen formal install" ;;
    *) sce_gate_fail "a recorded run is not as docs/formal/traces.json declares it" ;;
esac
