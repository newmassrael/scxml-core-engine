#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Configure the main CMake tree, for a caller that is about to run a ctest
# mutation round over it.
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

#
# Configured and not built. The round builds what its tests run and what those
# are built from (`mutation_ctest_closure_of_selection` in `scripts/mutate`),
# and a baseline that does not build stops the round with the build's own
# output, so nothing is left for a whole-tree build here to attribute. It used
# to build everything, because the round once discarded its build log; measured
# on run 39948952108 (2026-10-23), that build was 60 to 75 minutes of every
# C++ round job, spent mostly on targets the round never ran.
sce_main_build_dir
