#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: embed-vendor-smoke.yml
#
# embed/MANIFEST.json was emitted from the inputs it is committed with.
#
# The manifest is clang's reading of the public headers, so whether its
# content is right costs a package and a parse of every header, which
# `embed-vendor` does in CI. Whether it was emitted again after an input
# moved costs nothing of the kind: its stamp, embed/MANIFEST.inputs, records
# the git object id of each input, and HEAD's inputs are compared with it.
# The pre-commit hook runs the same comparison over the index, so this is the
# backstop for a commit made with the hook skipped.
#
# The inputs and the comparison are scripts/embed_manifest.py's.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 scripts/embed_manifest.py check --rev HEAD \
    || sce_gate_fail "embed manifest stamp: inputs moved without a regeneration"
