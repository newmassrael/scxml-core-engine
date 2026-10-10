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
# Sourced, never run. The reasoning for the date, and how to move it, is in
# `forge-rust.sh` beside the commands that use it.
QUEUE_SANITIZER_NIGHTLY="nightly-2026-10-08"
