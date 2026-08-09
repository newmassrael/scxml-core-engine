#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Spec-citation ledger validation over five workspaces (mirror of
# spec-citations.yml).
#
# Sub-second per workspace. This is the gate that was missing when the
# doc-hygiene rounds added cites without bindings and main went red
# post-push: the CI workflow validates every workspace, the hook did not.
# Same commands, same order; the migrate-citations form gates force every
# free-text cite through the token channel the validator inspects. The two
# directions matter together — the validator rejects a token with no
# binding, and the form gate rejects prose that evades the validator, so
# lowering a citation to free text is not an escape.
#
# The binary is REVISION-PINNED, not PATH-resolved. `~/.cargo/bin` is a
# shared slot that any `cargo install` silently overwrites, so a
# PATH-resolved `mnemosyne-cli` makes this mirror "whatever is installed"
# rather than the revision CI runs — and the two disagree in both
# directions. A gate that fails on a tree CI passes is as broken as one that
# passes on a tree CI fails. MNEMOSYNE_REV is read from spec-citations.yml
# so the pin keeps a single source of truth and a CI bump carries here
# automatically; this refuses to run under any other revision instead of
# reporting a verdict CI will not reproduce. Override the location (not the
# revision) with MNEMOSYNE_BIN.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

rev="$(sed -n 's/^[[:space:]]*MNEMOSYNE_REV:[[:space:]]*\([0-9a-f]\{40\}\).*/\1/p' \
    "$SCE_REPO_ROOT/.github/workflows/spec-citations.yml")"
[[ -n "$rev" ]] \
    || sce_gate_fail "no MNEMOSYNE_REV pin found in .github/workflows/spec-citations.yml"
short="${rev:0:8}"
root="${HOME}/.local/share/mnemosyne-rev/${short}"
BIN="${MNEMOSYNE_BIN:-${root}/bin/mnemosyne-cli}"
hint="cargo install --git https://github.com/newmassrael/mnemosyne --rev ${rev} --locked --root ${root} mnemosyne-cli"

[[ -x "$BIN" ]] \
    || sce_gate_fail "no rev-pinned mnemosyne-cli at ${BIN} — install it with: ${hint}"
have="$("$BIN" --version 2>&1 || true)"
[[ "$have" == *"$short"* ]] \
    || sce_gate_fail "${BIN} reports '${have}', expected revision ${short} — reinstall with: ${hint}"
sce_gate_step "pinned to ${have}"

for ws in docs/spec/scxml docs/sce-ledger/mesh docs/sce-ledger/wire docs/spec/synth docs/sce-ledger/bytesguard; do
    ( cd "$ws" \
        && "$BIN" validate-workspace >/dev/null \
        && "$BIN" validate-code-refs >/dev/null \
        && "$BIN" validate-verifies-linkage >/dev/null \
        && "$BIN" validate-content-drift >/dev/null ) \
        || sce_gate_fail "${ws} ledger validation"
done

python3 tools/mnemosyne-adoption/migrate_citations.py --check \
    --from-toml docs/spec/scxml/mnemosyne.toml >/dev/null \
    || sce_gate_fail "scxml citation-form gate"
python3 tools/mnemosyne-adoption/migrate_citations.py --check --namespace synth \
    --from-toml docs/spec/synth/mnemosyne.toml >/dev/null \
    || sce_gate_fail "synth citation-form gate"
python3 tools/mnemosyne-adoption/migrate_citations.py --check --namespace bytesguard \
    --from-toml docs/sce-ledger/bytesguard/mnemosyne.toml >/dev/null \
    || sce_gate_fail "bytesguard citation-form gate"

# Ledger-existence gate over the trees deliberately kept in PROSE. The form
# gates above cover only the dirs enrolled in a ledger's `paths`, which
# leaves the codegen templates and every backend/test tree free to cite a
# section number that does not exist — and templates are the propagation
# source: one fabricated number reached 168 sites through generated copies
# before this gate existed. Token form is NOT demanded here (template
# comments ship verbatim inside generated code and stay readable to
# consumers); only the claim that the cited section exists is enforced.
for tree in tools/codegen/templates tests backends examples web; do
    python3 tools/mnemosyne-adoption/migrate_citations.py "$tree" --check-ledger >/dev/null \
        || sce_gate_fail "ledger-existence gate: ${tree}"
done
