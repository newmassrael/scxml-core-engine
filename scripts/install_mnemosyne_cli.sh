#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the rev-pinned `mnemosyne-cli` where `scripts/gates/ledger-citations.sh`
# looks for it.
#
# Two lanes besides the citations one need this binary and neither had it:
# `gate_registry_contract` drives the *staged* citation stage as a subprocess,
# and that stage exits 3 — "the gate could not run, its own tooling is
# missing" — without the pin. Both `tree-hygiene` and `rust-workspace-tests`
# therefore failed on every push for months, saying nothing about the tree
# each time.
#
# A script rather than three copies of the same YAML: the revision has one
# home (`MNEMOSYNE_REV` in `.github/workflows/spec-citations.yml`, which the
# gate itself reads), and the install path it implies is derived here the way
# the gate derives it. A workflow that restated either would be a second
# reader of a fact that already drifts across six places.
#
# Idempotent: a binary already at the pinned revision is left alone, so a
# cache restore makes this a no-op rather than a rebuild.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
PIN_FILE="$REPO_ROOT/.github/workflows/spec-citations.yml"

rev="$(sed -n 's/^[[:space:]]*MNEMOSYNE_REV:[[:space:]]*\([0-9a-f]\{40\}\).*/\1/p' "$PIN_FILE")"
if [[ -z "$rev" ]]; then
    echo "install_mnemosyne_cli: no MNEMOSYNE_REV pin found in $PIN_FILE" >&2
    exit 1
fi

short="${rev:0:8}"
root="${HOME}/.local/share/mnemosyne-rev/${short}"
bin="${root}/bin/mnemosyne-cli"

# The revision is checked, not just the path: `cargo install --root` leaves a
# binary behind when the rev moves, and a stale one answers every question
# with the wrong ledger schema. The gate makes the same check before it runs.
if [[ -x "$bin" ]] && "$bin" --version 2>&1 | grep -q "$short"; then
    echo "install_mnemosyne_cli: $bin is already at $short"
    exit 0
fi

echo "install_mnemosyne_cli: installing mnemosyne-cli $short into $root"
cargo install --git https://github.com/newmassrael/mnemosyne \
    --rev "$rev" --locked --root "$root" mnemosyne-cli

"$bin" --version
