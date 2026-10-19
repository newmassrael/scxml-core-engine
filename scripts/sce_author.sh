#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The authoring tools' command line, from a checkout (the bundle's `bin/sce-author` is the same
# thing for a machine with no tree): extract, convert, check-conversion, assemble. Unlike the server's
# launcher it needs no generator built, since none of these reads a model.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PYTHONPATH="$repo_root/tools/authoring${PYTHONPATH:+:$PYTHONPATH}"
exec python3 -m sce_author "$@"
