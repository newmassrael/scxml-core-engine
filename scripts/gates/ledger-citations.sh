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
# 6357 files. It used to be the cheap half — the binding axes were 108.9s —
# and since the mnemosyne pin moved to c9b276bf (one symbol resolution per
# file rather than per citation) they are 2.6s, so this sweep is now the
# larger half of the gate.

# The rev-pinned binary, resolved once for both stages. Two stages reading
# the pin would be two readers of one fact, and the commit stage is the one
# that would drift silently — it runs on every commit and nobody watches its
# version line. The resolution is the pin's whole point: `~/.cargo/bin` is a
# shared slot any `cargo install` overwrites, so a PATH-resolved binary makes
# this gate "whatever is installed" rather than the revision CI runs.
sce_citation_binary() {
    local rev short root bin have hint
    rev="$(sed -n 's/^[[:space:]]*MNEMOSYNE_REV:[[:space:]]*\([0-9a-f]\{40\}\).*/\1/p' \
        "$SCE_REPO_ROOT/.github/workflows/spec-citations.yml")"
    [[ -n "$rev" ]] \
        || sce_gate_fail "no MNEMOSYNE_REV pin found in .github/workflows/spec-citations.yml"
    short="${rev:0:8}"
    root="${HOME}/.local/share/mnemosyne-rev/${short}"
    bin="${MNEMOSYNE_BIN:-${root}/bin/mnemosyne-cli}"
    hint="cargo install --git https://github.com/newmassrael/mnemosyne --rev ${rev} --locked --root ${root} mnemosyne-cli"
    # Both of these are the gate's own tooling, not the tree it judges, so they
    # exit 3 rather than 1 — see `sce_gate_cannot_run`.
    [[ -x "$bin" ]] \
        || sce_gate_cannot_run "no rev-pinned mnemosyne-cli at ${bin} — install it with: ${hint}"
    have="$("$bin" --version 2>&1 || true)"
    [[ "$have" == *"$short"* ]] \
        || sce_gate_cannot_run "${bin} reports '${have}', expected revision ${short} — reinstall with: ${hint}"
    printf '%s' "$bin"
}

# Run one validator in one workspace, and report what it reported.
#
# The tool collapses every failure into exit 1. Measured 2026-08-12 against the
# pinned binary: a missing `mnemosyne.toml`, a `--paths` value outside the
# workspace and an unknown flag all exit 1, exactly as a genuine unbound
# citation does. A status therefore does not carry a cause, and a gate that
# names one is stating something it did not measure — which is not a wording
# problem, because the author is the one who reads it. This gate did exactly
# that: when the mnemosyne pin was raised and the binary was not yet installed
# under the new revision's path, the run ended with "a staged file carries a
# citation whose binding does not hold". The citations were fine.
#
# So the verdict is left to the only party that measured it. The tool's own
# output is surfaced instead of discarded, and this gate says what it knows:
# which workspace, which validator, which status.
#
# `$BIN` is resolved by the caller, in a shell that can act on the failure. It
# used to be resolved here, per workspace, inside a command substitution inside
# a subshell — where `sce_gate_fail` ends the substitution and nothing else, so
# the empty expansion ran as a command and its 127 arrived as an author's fault.
sce_citation_run() {
    local ws="$1" out rc
    shift
    [[ -n "${BIN:-}" ]] \
        || sce_gate_fail "internal: ${ws}: \`$1\` was reached before the validator was resolved"
    out="$( cd "$SCE_REPO_ROOT/$ws" && "$BIN" "$@" 2>&1 )" && rc=0 || rc=$?
    if (( rc == 0 )); then
        return 0
    fi
    printf '%s\n' "$out" >&2
    sce_gate_fail "${ws}: \`$1\` exited ${rc} — its own report is above"
}

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
# It runs the binding axes too, since the mnemosyne pin moved to c9b276bf:
# `validate-code-refs --paths` scopes them to a file list and states that a
# scoped answer equals the full answer restricted to those files. Before that
# surface existed they were push-only, and this comment said so — the reason
# was true and stopped being true, which is why it is written here as a
# measurement rather than as a property of the axes.
#
# What stays at push are the SPEC-side axes (binding_unbacked, impl_missing,
# verification_missing, misclassified_coverage, blanket_verifies): they ask
# whether a claim in the store has a witness ANYWHERE, which no file list can
# answer. The tool names them as not judged on every scoped run rather than
# reporting them as zero, so that boundary is its statement, not ours.
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

    # ── Binding axes, scoped to the staged files ──────────────────
    #
    # This half used to be push-only, and the reason was written down: the
    # binding axes need the validator's whole-tree symbol resolution, and the
    # tool had no way to ask about a file list. `validate-code-refs --paths`
    # (mnemosyne 4-B) is that way, and its contract is the one this needs — a
    # scoped run's answer equals the full run's answer restricted to those
    # files, and every axis it did NOT judge is named rather than reported as
    # zero. So the axes that stay at push stay there by the tool's own
    # statement, not by ours.
    #
    # Why a second scope split. `--paths` reads the WORKING TREE; the existence
    # half above reads the staged content. For a file whose worktree copy
    # differs from what is staged, a green answer here would be an answer about
    # text that is not being committed — the exact hole `git checkout-index`
    # closes above. Those files are therefore not asked about, and are NAMED,
    # which is the same rule the tool applies to its own unjudged axes: a count
    # of zero has to mean measured-and-clean.
    mapfile -t dirty < <(git diff --name-only --diff-filter=ACMR)
    decidable=()
    undecidable=()
    for path in "${scoped[@]}"; do
        if printf '%s\n' "${dirty[@]}" | grep -qxF -- "$path"; then
            undecidable+=("$path")
        else
            decidable+=("$path")
        fi
    done
    if (( ${#undecidable[@]} > 0 )); then
        printf 'staged citation check: binding axes not judged for %d file(s) — their worktree copy differs from what is staged, so an answer here would not be about the commit (the push gate judges them):\n' \
            "${#undecidable[@]}" >&2
        printf '  %s\n' "${undecidable[@]}" >&2
    fi
    # A zero-length `--paths` is rejected by the tool, and rightly: an empty
    # scope reads as "everything" to one reader and "nothing" to the next.
    if (( ${#decidable[@]} > 0 )); then
        abs=("${decidable[@]/#/$SCE_REPO_ROOT/}")
        repo_real="$(readlink -f "$SCE_REPO_ROOT")"
        # Resolved here, once, in the shell that can end the run — not per
        # workspace inside the loop below, where its refusal had no way out.
        # A commit with nothing decidable never reaches this line: those files
        # are named for the push gate, so demanding the tool to judge nothing
        # would fail a commit over a verdict that was never going to be given.
        BIN="$(sce_citation_binary)"
        for ws in docs/spec/scxml docs/sce-ledger/mesh docs/sce-ledger/wire \
                  docs/spec/synth docs/sce-ledger/bytesguard; do
            # A ledger reached through a symlink belongs to whatever repository
            # holds it, and the tool says so rather than guessing: it rejects a
            # `--paths` value outside its own workspace. That is the right
            # answer — a store in another checkout cannot judge this one's
            # staged files — so the pairing is checked here instead of being
            # discovered as an error. The self-test fixtures are exactly this
            # shape, a synthetic repo with `docs` symlinked in.
            if [[ "$(readlink -f "$SCE_REPO_ROOT/$ws")" != "$repo_real"/* ]]; then
                printf 'staged citation check: %s resolves outside this repository, so it judges none of its staged files\n' \
                    "$ws" >&2
                continue
            fi
            # Absolute paths, because the tool resolves `--paths` against its
            # own working directory and this loop moves between five of them.
            # A staged file outside a workspace's configured `paths` comes back
            # classified as out-of-read-set, not as a violation, so every
            # workspace can be handed the whole list.
            sce_citation_run "$ws" validate-code-refs --paths "${abs[@]}"
        done
    fi
    exit 0
fi

BIN="$(sce_citation_binary)"
sce_gate_step "pinned to $("$BIN" --version 2>&1)"

# One call per validator rather than a `&&` chain: chained, the four shared a
# single failure message and a single discarded output, so a red said "${ws}
# ledger validation" and the author had neither the axis nor the report. The
# order is unchanged and so is the stop-at-first-failure behaviour, since the
# helper ends the run.
for ws in docs/spec/scxml docs/sce-ledger/mesh docs/sce-ledger/wire docs/spec/synth docs/sce-ledger/bytesguard; do
    for check in validate-workspace validate-code-refs validate-verifies-linkage \
                 validate-content-drift; do
        sce_citation_run "$ws" "$check"
    done
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
