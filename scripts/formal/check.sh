#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Check every TLA+ model under docs/formal, as docs/formal/models.json declares them
# (docs/adr/0115, D5). The gate is scripts/gates/formal-models.sh; this is the same
# judge for a person to run, with its options: `check.sh --only OpenInstances`.
# Among the models it judges is the bounded broadcast log of RFC §synth-5-S
# (docs/formal/BroadcastLog.tla, ADR 0062).
set -euo pipefail

exec python3 "$(dirname "${BASH_SOURCE[0]}")/models.py" "$@"
