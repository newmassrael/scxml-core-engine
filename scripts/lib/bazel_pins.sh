# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Bazel versions SCE's Bazel channels are built against, in one place.
#
# - SCE_BAZEL_RULES_CC_VERSION: the rules_cc the generated BUILD files load
#   their C++ rules from. Bazel 9 removed the native cc_library, so every BUILD
#   file SCE writes loads `@rules_cc//cc:defs.bzl`; the embed package's
#   MODULE.bazel names this version (scripts/package_embed.sh).
# - SCE_BAZEL_VERSION: the Bazel the consumer smoke builds with
#   (scripts/smoke_bazel_consumer.sh, through bazelisk's .bazelversion). The
#   newest major, since it is the one that refuses what an older one accepts.
#
# Sourced, not executed.

SCE_BAZEL_RULES_CC_VERSION="0.2.27"
SCE_BAZEL_VERSION="9.3.1"

# sce_bazel_select: put the pinned Bazel bx installs on a build machine
# (`[platform.linux.tools.bazel]` in bx.toml, scripts/install_bazel.sh) first on
# PATH when it is there. Where it is not (a hosted runner, a workstation) the
# `bazel` already on PATH runs, bazelisk reading the consumer's .bazelversion.
sce_bazel_select() {
    local bin="$HOME/.local/share/bazel/$SCE_BAZEL_VERSION/bin"
    if [[ -x "$bin/bazel" ]]; then
        PATH="$bin:$PATH"
        export PATH
    fi
}
