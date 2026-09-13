#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Single source of truth, from shell, for the Mnemosyne revision this
# repository is pinned to and the binaries a revision root must carry.
#
#   REPO_ROOT="$(git rev-parse --show-toplevel)"
#   source "$REPO_ROOT/scripts/lib/mnemosyne_pin.sh"
#   rev="$(mnemosyne_pin_rev "$REPO_ROOT")"
#   root="$(mnemosyne_pin_root "$rev")"
#
# WHY THIS FILE EXISTS — measured 2026-09-14, during the bump to df1f17ce.
#
# The pin is a REVISION-level fact: `[tool] pin` in each `mnemosyne.toml`
# says which revision may act on that workspace, and a binary carrying a
# different stamp execs into `${MN_ROOT:-$HOME/.local/mn}/<pin>/bin/<same
# name>` rather than guessing. But procurement and verification were both
# BINARY-level: `cargo install … mnemosyne-cli` installs one binary, and
# the citation gate checked that one binary. So a revision root could hold
# `mnemosyne-cli` and nothing else, and every check in this repository still
# read green.
#
# It did. `~/.local/share/mnemosyne-rev/ecee1fe0/bin/` carried the CLI alone,
# so the MCP servers configured against these workspaces execed toward a
# `mnemosyne-mcp` that was not there. The refusal reached nobody: an stdio
# MCP server that exits is reported by the harness as `CONNECTION_CLOSED`,
# naming no revision and no install line — where the CLI's identical failure
# says "no rev-pinned mnemosyne-cli at … install it with: …". Same event,
# one half of it audible.
#
# The repair is to make a half-filled root unreachable rather than detected:
# one list, one install command, one verifier. `MNEMOSYNE_PIN_BINARIES` is
# that list, and it is the only place a consumer of this revision is named.
#
# ⚠ The workflow stays the SSOT for the revision itself. This file reads it;
# it never restates it. A second spelling of the revision is the defect this
# file exists to remove, not one to introduce.

# Every binary a revision root must carry for this repository's consumers.
#
# `mnemosyne-cli`  the gates, the hooks and the adoption tests.
# `mnemosyne-mcp`  the MCP servers configured against the five workspaces.
#                  Its write tools migrate a store to the writing binary's
#                  schema, so an unpinned one is one write away from a store
#                  the pinned CI binary cannot open.
MNEMOSYNE_PIN_BINARIES=(mnemosyne-cli mnemosyne-mcp)

# The 40-hex revision, read from the one place that declares it.
mnemosyne_pin_rev() {
    local repo_root="$1" pin_file rev
    pin_file="$repo_root/.github/workflows/spec-citations.yml"
    rev="$(sed -n 's/^[[:space:]]*MNEMOSYNE_REV:[[:space:]]*\([0-9a-f]\{40\}\).*/\1/p' \
        "$pin_file")"
    if [[ -z "$rev" ]]; then
        echo "mnemosyne_pin: no MNEMOSYNE_REV pin found in $pin_file" >&2
        return 1
    fi
    printf '%s' "$rev"
}

# The revision-keyed install root. The directory's name is the 8-character
# form, which is also the string `[tool] pin` carries and the one
# `--version` prints, so all three comparisons are the same comparison.
mnemosyne_pin_root() {
    printf '%s' "${HOME}/.local/share/mnemosyne-rev/${1:0:8}"
}

# The command that fills a root. One `cargo install` for every binary, so the
# root is complete or absent — never the half that reads green.
mnemosyne_pin_install_hint() {
    local rev="$1" root
    root="$(mnemosyne_pin_root "$rev")"
    printf 'cargo install --git https://github.com/newmassrael/mnemosyne --rev %s --locked --root %s %s' \
        "$rev" "$root" "${MNEMOSYNE_PIN_BINARIES[*]}"
}

# Names the binaries a root is missing or holding at the wrong revision,
# one per line; prints nothing and returns 0 when the root is complete.
#
# The revision is checked, not just the path: `cargo install --root` leaves
# the old binary behind when the rev moves, and a stale one answers every
# question with another schema's judgement.
mnemosyne_pin_missing() {
    local rev="$1" short root bin have
    short="${rev:0:8}"
    root="$(mnemosyne_pin_root "$rev")"
    for bin in "${MNEMOSYNE_PIN_BINARIES[@]}"; do
        if [[ ! -x "$root/bin/$bin" ]]; then
            printf '%s: absent\n' "$bin"
            continue
        fi
        have="$("$root/bin/$bin" --version 2>&1 || true)"
        [[ "$have" == *"$short"* ]] || printf '%s: reports %s\n' "$bin" "$have"
    done
}
