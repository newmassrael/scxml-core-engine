#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Configure and build the main CMake tree, for a caller that is about to run
# something which needs one.
#
# `mutation-rounds` refuses (exit 3) when a selected casefile drives its round
# through ctest and `build/` is not configured: there is nothing to select from
# and nothing to rebuild, and reporting green over that would be the silent
# coverage loss the gate exists to remove. That refusal is the right answer for
# a developer — configuring and building this tree is half an hour, and a gate
# that started spending it on their behalf would be deciding something that is
# theirs to decide — which leaves exactly one party needing a step that does it:
# CI.
#
# The step lived in the lane's YAML and both halves of it were wrong. It
# configured without building `sce-codegen` first, which cannot work on a
# runner (`cmake/SCEFindCodegen.cmake` stops with a FATAL_ERROR), and it
# configured `CMAKE_BUILD_TYPE=Debug` against the `RelWithDebInfo` every gate
# requires — `sce_main_build_dir` refuses any other build type in as many
# words, so the lane would have spent that half-hour producing a tree the gate
# it serves then declined to judge. Neither was noticed because nothing had run
# it: the lane's ctest path was first reached on 2026-08-17, 34 commits after
# the casefiles that need it were written.
#
# So the recipe is here, once, and both callers reach it by name: the lane runs
# this script, and the gate's refusal message tells a developer to run the same
# one. A tree that CI builds and a tree a developer builds differing in build
# type is not a difference a suite can see — it is a difference in what the
# suite was measuring.
#
#     scripts/prepare_ctest_tree.sh
#
# Honours `SCE_W3C_BUILD_DIR` because `sce_main_build_dir` does; without it the
# tree is `build/`, which is the directory the casefiles' ctest selectors name.

source "$(dirname "${BASH_SOURCE[0]}")/gates/lib.sh"

sce_main_build_dir

# Built here rather than left to the first round, and with its output on the
# terminal. `scripts/mutate` builds its own baseline with stdout and stderr
# discarded — deliberately, because a round's interesting output is the test
# run — so a tree that does not compile at all would surface there as a round
# that proves nothing rather than as the build failure it is. That attribution
# is what a first build in the open buys.
sce_gate_step "building $SCE_MAIN_BUILD_DIR"
cmake --build "$SCE_MAIN_BUILD_DIR" --parallel "$(nproc)" \
    || sce_gate_fail "the main tree does not build; the rounds below it would have nothing to judge"
