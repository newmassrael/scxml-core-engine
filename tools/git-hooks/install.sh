#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Activate the tracked git hooks under tools/git-hooks/ for the current
# clone by pointing git's core.hooksPath at this directory.
#
# Why not symlink into .git/hooks/? Because .git/hooks is per-clone and
# opaque; newcomers cloning the repo would silently get no hooks unless
# told to run an install step. Setting core.hooksPath is a single-line
# config change that git itself reads on every invocation, so hooks stay
# in lock-step with whatever is checked out (branch switches, rebases,
# pulls). This matches the husky / Kubernetes / Rust-analyzer layout.
#
# Run once per clone:
#     bash tools/git-hooks/install.sh
#
# To opt out of a single push (emergencies only):
#     SKIP_PREPUSH=1 git push ...

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

HOOKS_DIR="tools/git-hooks"

if [[ ! -d "$HOOKS_DIR" ]]; then
    echo "install: $HOOKS_DIR not found — run from inside the SCE repo" >&2
    exit 1
fi

git config core.hooksPath "$HOOKS_DIR"
echo "install: git core.hooksPath = $HOOKS_DIR"
echo "install: active hooks:"
for hook in "$HOOKS_DIR"/*; do
    [[ -x "$hook" ]] || continue
    name="$(basename "$hook")"
    [[ "$name" == "install.sh" ]] && continue
    printf '  - %s\n' "$name"
done
