#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: spec-citations.yml
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

# The existence axis has no tree list any more, and that is the point. It used
# to name five trees, and a citation's visibility then depended on two lists at
# once: this one, and the scanner's set of file extensions. Measured 2026-08-11:
# `web/` was named here and contributed 0 of its 46 tracked files, because the
# scanner only read extensions it could REWRITE — so seven fabricated section
# numbers were "checked" at every push. Nine more sat outside the five trees
# entirely, in `SCE_MESH.md`, a `pyproject.toml`, and the CI workflows.
#
# Scope is now every tracked file, decided by `git ls-files` rather than by a
# suffix or a directory anyone has to remember to extend. Measured at 4.4s over
# 6341 files, against 108.9s for the binding axes this gate also runs, so the
# widening is free.

# ── Staged-scope mode (tools/git-hooks/pre-commit) ────────────────
#
# The existence half of this gate, over the staged content only.
#
# Why it exists: the full gate costs ~110s and runs at push, so a fabricated
# citation is reported once for a whole batch of commits and nothing says which
# commit introduced it. Measured on this repo twice — thirteen cites across
# four commits rejected in one push (2026-08-10), and a fabricated `§synth-F4`
# rejected at push after the round that wrote it had already been committed
# (2026-08-11). Both would have been one line at one commit.
#
# What it does NOT do, deliberately: the binding axes (citation_unbound,
# symbol_mismatch, binding_unbacked) need the validator's whole-tree symbol
# resolution — 108.9 of the gate's 110 seconds, measured — and cannot be scoped
# to a diff. Those stay at push. Existence is the axis that IS decidable from
# the staged content alone, and it is the hallucination class.
#
# Staged CONTENT, not the working tree: `git checkout-index` materialises the
# index, so a citation fixed-but-not-staged cannot pass the gate that is about
# to commit the unfixed version.
if [[ "${1:-}" == "--staged" ]]; then
    # Every staged path, matching the full gate's scope exactly. The two used to
    # share a tree list so they could not disagree about coverage; now they
    # share the absence of one, which is the same guarantee without a list to
    # keep current. Non-text files cost nothing: the checker decides at the read.
    mapfile -t scoped < <(git diff --cached --name-only --diff-filter=ACMR)
    if [[ ${#scoped[@]} -eq 0 ]]; then
        exit 0
    fi
    work="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$work'"
    git checkout-index --prefix="$work/" -- "${scoped[@]}"
    # The tool resolves its ledgers through its own installed location, so it
    # reads the repo's stores while scanning the materialised index, and
    # `--report-root` makes it name the file the author has to open.
    #
    # Exit status is read, not just tested: 3 means the checker could not run
    # (an unreadable store), and reporting that as a bad citation would blame
    # the author's text for a fault in the gate's own inputs.
    set +e
    python3 "$SCE_REPO_ROOT/tools/mnemosyne-adoption/migrate_citations.py" \
        --check-ledger --report-root "$work" \
        "${scoped[@]/#/$work/}"
    status=$?
    set -e
    case "$status" in
        0) ;;
        1) sce_gate_fail "staged citation names a section absent from the ledger" ;;
        *) sce_gate_fail "the staged citation check could not run (exit ${status})" ;;
    esac
    exit 0
fi

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

# Ledger-existence gate over every tracked file. The form gates above cover
# only the dirs enrolled in a ledger's `paths`, which leaves the codegen
# templates, every backend/test tree, the design docs and the CI workflows free
# to cite a section number that does not exist — and templates are the
# propagation source: one fabricated number reached 168 sites through generated
# copies before this gate existed. Token form is NOT demanded here (template
# comments ship verbatim inside generated code and stay readable to
# consumers); only the claim that the cited section exists is enforced —
# for a prose cite and, since the token pass landed, for one already written
# as `§<ns>-<id>`. The gate promised the second in its own error text long
# before it checked it, so a fabricated id typed straight in §-form passed
# every reader.
#
# One sweep from the repo root, not a loop over trees: the checker reports how
# many files it read, and a per-tree loop turns that one number into a set of
# numbers nobody compares. A path that yields nothing is an error there, so a
# scan set that silently shrinks cannot come back as "OK".
python3 tools/mnemosyne-adoption/migrate_citations.py . --check-ledger \
    || sce_gate_fail "ledger-existence gate"
