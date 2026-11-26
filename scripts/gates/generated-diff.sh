#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: generated-diff.yml
#
# The generated diff of a change (ADR 0097, section 6): every tree
# tools/codegen/trees/ declares, materialized at two commits by
# scripts/generated_diff.sh, one patch per tree and a summary, written to
# target/generated-diff/. Its verdict is that script's: the head materializes
# every tree it declares. The base side is rendered where it can be and only
# noted where it cannot, since an older commit may declare a tree the generator
# of its day could not materialize.
#
# The commits are SCE_GENERATED_DIFF_BASE and SCE_GENERATED_DIFF_HEAD, which
# the lane resolves (the last commit it published for, or a pull request's
# base); without them the base is HEAD's first parent and the head is HEAD,
# the diff of the last commit, as a developer or a full run asks for it.
#
# A gate rather than a step of the lane's own, because the generations need
# the host image's packages (libxml2 for the generator, clang-format-19 for its
# C++ trees, Go for gofmt), and a gate is how a lane enters the image
# (docs/adr/0145).

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

head="${SCE_GENERATED_DIFF_HEAD:-HEAD}"
base="${SCE_GENERATED_DIFF_BASE:-}"
if [[ -z "$base" ]]; then
    base="$(git rev-parse --verify --quiet "$head^1")" \
        || sce_gate_cannot_run "$head has no parent and no SCE_GENERATED_DIFF_BASE names a base, so there is no diff to render"
fi
out="$SCE_REPO_ROOT/target/generated-diff"
rm -rf "$out"
sce_gate_step "rendering the generated diff of $base..$head into target/generated-diff/"
"$SCE_REPO_ROOT/scripts/generated_diff.sh" --out "$out" "$base" "$head" \
    || sce_gate_fail "the head does not materialize every tree it declares (target/generated-diff/ holds each tree's log)"
