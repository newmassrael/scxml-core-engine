#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Launch the rev-pinned `mnemosyne-mcp` for one of this repository's ledger
# workspaces. Point an MCP client's `command` at THIS FILE, never at a
# revision-keyed path:
#
#   "command": "/home/coin/scxml-core-engine/scripts/mnemosyne_mcp.sh",
#   "args":    ["--workspace", "/home/coin/scxml-core-engine/docs/spec/scxml"]
#
# WHY — measured 2026-09-14.
#
# An MCP client's configuration lives outside this repository (for Claude
# Code, `~/.claude.json`). When its `command` names
# `…/mnemosyne-rev/<rev>/bin/mnemosyne-mcp`, that path is a THIRD copy of the
# pin, beside `MNEMOSYNE_REV` in the workflow and `[tool] pin` in the five
# `mnemosyne.toml` files — and it is the copy no gate can read and no bump
# updates. The three servers configured here sat two revisions behind for
# exactly that reason.
#
# Naming this script instead removes that copy. The revision is resolved at
# launch from the workflow, which is where a bump already lands, so the bump
# carries to the MCP with no second edit — the property the README claims for
# `pre-push` Stage 8, now true of the MCP too.
#
# The second thing this buys is a VOICE. An stdio MCP server that exits
# reaches the client as `CONNECTION_CLOSED`: no revision, no install line,
# nothing to act on. stderr does not become a notification (this tree has
# learned that elsewhere), so a refusal is written to a file as well, and the
# client is told where to look.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$REPO_ROOT/scripts/lib/mnemosyne_pin.sh"

LOG_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/sce-mnemosyne-mcp"
LOG="$LOG_DIR/launch.log"
mkdir -p "$LOG_DIR"

# Both channels, always: the file is what survives a client that discards
# stderr, and stderr is what a human running this by hand reads.
refuse() {
    local msg="$1"
    printf '%s mnemosyne_mcp: %s\n' "$(date -Is)" "$msg" >>"$LOG"
    printf 'mnemosyne_mcp: %s\n' "$msg" >&2
    printf 'mnemosyne_mcp: this refusal is also in %s\n' "$LOG" >&2
    exit 1
}

rev="$(mnemosyne_pin_rev "$REPO_ROOT")" || refuse "no MNEMOSYNE_REV pin in the workflow"
root="$(mnemosyne_pin_root "$rev")"

missing="$(mnemosyne_pin_missing "$rev")"
if [[ -n "$missing" ]]; then
    # Joined on the newlines the reporter emits, not by word splitting: each
    # line is "<binary>: <what is wrong with it>" and splitting it on spaces
    # turned "mnemosyne-mcp: absent" into two findings that read as neither.
    refuse "revision root $root is incomplete (${missing//$'\n'/; }) — install it with: $REPO_ROOT/scripts/install_mnemosyne_cli.sh"
fi

# `MN_ROOT` so that a workspace whose `[tool] pin` names another revision
# resolves it under the layout this repository documents, rather than the
# upstream default `$HOME/.local/mn` — where this tree installs nothing, and
# where an absent build is what the exec hop dies on.
export MN_ROOT="${MN_ROOT:-$HOME/.local/share/mnemosyne-rev}"

printf '%s mnemosyne_mcp: exec %s %s\n' "$(date -Is)" "$root/bin/mnemosyne-mcp" "$*" >>"$LOG"
exec "$root/bin/mnemosyne-mcp" "$@"
