#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: clang-format-check.yml
#
# Every tracked C and C++ file is formatted as .clang-format says, by
# clang-format of the major the tree was normalized with (19). The file
# enumeration and the policy for generated sources are
# scripts/check_clang_format.sh's, which the commit hook runs on the staged
# files (tools/git-hooks/pre-commit) and this runs on the whole tree.
#
# A gate rather than a step of the lane's own, because the formatter is a
# package of the host image the gate runs in (tools/ci-images/host,
# docs/adr/0145), and a gate is how a lane enters the image: the lane installed
# clang-format-19 from the live archive at job time until 2026-11-26, so the
# patch release that judged a file was whatever the archive held that day.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v clang-format-19 >/dev/null 2>&1 \
    || sce_gate_cannot_run "clang-format-19 is not on PATH; the host image carries it (tools/ci-images/host/Containerfile)"
sce_gate_step "$(clang-format-19 --version)"

CLANG_FORMAT=clang-format-19 "$SCE_REPO_ROOT/scripts/check_clang_format.sh" \
    || sce_gate_fail "a C or C++ file is not formatted as .clang-format says (clang-format-19 --dry-run --Werror, above); clang-format-19 -i <file> formats it"
