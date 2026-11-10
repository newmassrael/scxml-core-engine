# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The nightly toolchain the queue runtime's sanitizer runs are pinned to (SCE
# Protocol-Synthesis RFC §synth-5-P, verification layer 5), in one place.
#
# Two things read it: `scripts/gates/forge-rust.sh`, which runs the real-thread
# tests under Miri and ThreadSanitizer, and the mutation casefile that has
# ThreadSanitizer judge a weakened hazard-pointer publication
# (`sce-build/tests/mutations/a_hazard_is_published_before_it_is_checked.cases`).
# A date spelled in each would be two answers to one question, and the day
# they differ the casefile would be judging a build the gate does not run.
#
# The date is the repository's one nightly, `SCE_RUST_NIGHTLY` in
# scripts/lib/rust_toolchains.sh, which bx.toml declares with Miri and rust-src
# (`[platform.linux.tools.rust-nightly]`) and which the threaded emscripten tree
# builds with too: a second pin of the same date would be a second table at the
# same place. Moving it moves both uses, so both are run again on the new date.
#
# Sourced, never run. The reasoning for the date, and how to move it, is in
# `forge-rust.sh` beside the commands that use it.

# shellcheck source=rust_toolchains.sh
source "$(dirname "${BASH_SOURCE[0]}")/rust_toolchains.sh"
QUEUE_SANITIZER_NIGHTLY="$SCE_RUST_NIGHTLY"
