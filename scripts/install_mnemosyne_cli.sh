#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the rev-pinned Mnemosyne build where this repository looks for it.
#
# The unit is the REVISION, not a binary: every name in
# `MNEMOSYNE_PIN_BINARIES` goes into one revision root, because a root
# holding some of them passed every check this repository had while the MCP
# servers it was supposed to serve died at start-up. The list, the revision
# and the verifier all live in `scripts/lib/mnemosyne_pin.sh`; the file name
# here still says `cli` because the workflows and docs that call it do.
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
#
# ⚠ THE BUILD MACHINES NEED IT TOO, and nothing reminds anyone. `bx` can send
# `scripts/gate ledger-citations` to a host in the fleet, and that host looks
# for the binary at the same derived path this script writes. Its `needs`
# check cannot see the difference: `command -v mnemosyne-cli` answers yes for
# ANY revision, so a machine carrying the wrong one is chosen and the gate
# then exits with an install line instead of a verdict — which is not green,
# however it reads. Measured 2026-08-24: all three registered hosts carried
# either no pinned build or a different revision.
#
# So a pin bump is four installs, not one: here, and once on each host. There
# is deliberately no automation for it in this script — reaching into a
# machine registry is not a repository's business, and a script that tried
# would go stale the first time a host was added.
#
# ⚠ AND A HOST THAT IS DOWN AT BUMP TIME KEEPS THE OLD REVISION. Measured
# 2026-09-14, bumping to `df1f17ce`: `pc2` and `pc3` took the install and
# `pc4` did not answer at all, so it still carries whatever it had. Nothing
# notices until something is sent there, and then the symptom is this
# script's install line arriving where a verdict was expected. Running this
# script on a host is idempotent, so the remedy on any doubt is to run it
# rather than to check first.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$REPO_ROOT/scripts/lib/mnemosyne_pin.sh"

rev="$(mnemosyne_pin_rev "$REPO_ROOT")"
short="${rev:0:8}"
root="$(mnemosyne_pin_root "$rev")"

# A REVISION is installed, not a binary. Both names go to one `cargo install`
# so the root is complete or absent, never the half that reads green — see
# the header of `scripts/lib/mnemosyne_pin.sh` for what the half cost.
missing="$(mnemosyne_pin_missing "$rev")"
if [[ -z "$missing" ]]; then
    echo "install_mnemosyne_cli: $root already carries ${MNEMOSYNE_PIN_BINARIES[*]} at $short"
    exit 0
fi

echo "install_mnemosyne_cli: $root needs:"
while IFS= read -r line; do printf '  %s\n' "$line"; done <<<"$missing"
echo "install_mnemosyne_cli: installing ${MNEMOSYNE_PIN_BINARIES[*]} $short into $root"
cargo install --git https://github.com/newmassrael/mnemosyne \
    --rev "$rev" --locked --root "$root" "${MNEMOSYNE_PIN_BINARIES[@]}"

# Verified after installing, not assumed from exit 0: `cargo install` succeeds
# for a package whose binary lands under a name this repository does not ask
# for, and that root would still be half-filled.
still="$(mnemosyne_pin_missing "$rev")"
if [[ -n "$still" ]]; then
    echo "install_mnemosyne_cli: install finished and the root is still incomplete:" >&2
    while IFS= read -r line; do printf '  %s\n' "$line" >&2; done <<<"$still"
    exit 1
fi
for bin in "${MNEMOSYNE_PIN_BINARIES[@]}"; do
    "$root/bin/$bin" --version
done
