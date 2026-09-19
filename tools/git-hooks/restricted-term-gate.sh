#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Refuse a commit whose staged content carries a restricted term. Sourced by
# `pre-commit`, which calls `restricted_term_gate_staged`.
#
# ⚠⚠ THE TERM LIST IS NOT IN THIS REPOSITORY, AND MUST NEVER BE.
#
# The words this gate looks for are themselves the material it keeps out. A
# list of them committed here would be the disclosure it exists to prevent —
# the checker would publish, verbatim, the strings it was written to stop. So
# this file holds the MECHANISM and nothing else: it reads its patterns from a
# path OUTSIDE any git repository, defaulting to `~/.claude/restricted-terms.txt`
# (confirmed 2026-09-18 to have no `.git` at any ancestor).
#
# For the same reason this gate NEVER PRINTS WHAT IT MATCHED. A hook's output
# reaches terminals, CI logs and pasted bug reports; echoing the offending
# string there would move the disclosure rather than stop it. It prints the
# file, the line, and the INDEX of the pattern that fired. Look the index up in
# your own term file.
#
# ⚠ FAIL CLOSED. A missing or unreadable term file refuses the commit. The
# alternative — passing when the list cannot be read — makes the gate report
# success in precisely the situation where it checked nothing, which is the
# failure mode this repository has recorded more than once in other lanes.
#
# ⚠⚠⚠ THIS RUNS BEFORE `SKIP_PRECOMMIT`. Every other stage in `pre-commit` is
# bypassable, and rightly so: a formatting stage refuses something a later
# commit can fix. This one does not, because the remote is PUBLIC and a push
# does not un-publish — the same reason `ident-gate.sh` sits where it does. A
# formatting emergency is not a reason to disable a permanent-disclosure guard,
# so the bypass for this gate is its own variable and it announces itself.
#
# ⚠ It is only as wide as its patterns. A clean result means "no pattern
# matched", never "there is nothing restricted here". Finding something the
# gate missed is a reason to add a line to the term file, not to distrust the
# gate.

RESTRICTED_TERMS_FILE="${SCE_RESTRICTED_TERMS:-$HOME/.claude/restricted-terms.txt}"

restricted_term_gate_staged() {
    if [[ "${SCE_RESTRICTED_OVERRIDE:-0}" == "1" ]]; then
        printf '\n⚠ pre-commit: SCE_RESTRICTED_OVERRIDE=1 — restricted-term gate DISABLED.\n' >&2
        printf '  The remote is public and a push does not un-publish. You are\n' >&2
        printf '  asserting that you have read the staged content yourself.\n\n' >&2
        return 0
    fi

    if [[ ! -r "$RESTRICTED_TERMS_FILE" ]]; then
        printf '\nERROR pre-commit: restricted-term gate cannot read its term list.\n' >&2
        printf '  Expected at: %s\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  This gate FAILS CLOSED: a list it cannot read is a check it did\n' >&2
        printf '  not perform, and reporting success for that is worse than\n' >&2
        printf '  refusing. Create the file (one extended regex per line, `#` for\n' >&2
        printf '  comments) or point SCE_RESTRICTED_TERMS at it.\n' >&2
        printf '  ⚠ Keep it OUTSIDE every git repository.\n' >&2
        return 1
    fi

    # ⚠⚠ KNOWN WEAKNESS, MEASURED 2026-09-18 AND DELIBERATELY NOT PATCHED
    # HERE. Several of the list's patterns are anchored `\b<term>`, and a
    # word boundary needs a non-word character on the other side — so a
    # term does not match when one letter is stuck in front of it. That
    # is not a hypothetical spelling: the platform model this repository
    # is used against writes the same identifiers as member variables
    # with exactly such a prefix, so the shape most likely to be
    # committed is the shape that passes.
    #
    # Two repairs were tried in the gate and both were rejected BY
    # MEASUREMENT rather than by argument:
    #
    #   * Dropping the anchors from every pattern closes the hole and
    #     turns four of this repository's own committed files red — a
    #     vendored JSON header, a W3C specification snapshot, a solution
    #     file and a benchmark, none of which can hold a restricted term.
    #     A gate that refuses every commit is an off gate with extra
    #     steps.
    #   * Dropping them only for long plain identifiers keeps the tree
    #     green and closes nothing: the anchored patterns are not that
    #     shape.
    #
    # ⚠ A SECOND WEAKNESS OF THE SAME FAMILY, measured the same day: the
    # patterns are CASE-SENSITIVE. A term written one way is caught, and
    # the same identifier lower-cased or upper-cased is not. That matters
    # because a lower-cased spelling is not exotic — it is what any
    # normalising step produces, so a tool that folds case before writing
    # a file hands the gate something it will wave through.
    #
    # What is left needs a decision per pattern, which needs READING the
    # list — and reading it copies every restricted term into whatever
    # is doing the reading, which is worse than the hole. So both belong
    # to whoever owns the list, and this note is here so the weaknesses
    # are recorded rather than rediscovered.
    local -a pats=()
    local line
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ -z "${line// /}" ]] && continue
        [[ "$line" == \#* ]] && continue
        pats+=("$line")
    done < "$RESTRICTED_TERMS_FILE"

    if [[ ${#pats[@]} -eq 0 ]]; then
        printf '\nERROR pre-commit: the restricted-term list is empty (%s).\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  An empty list would pass every commit while looking like a check.\n' >&2
        return 1
    fi

    local -a staged=()
    mapfile -t staged < <(git diff --cached --name-only --diff-filter=ACMR)
    [[ ${#staged[@]} -eq 0 ]] && return 0

    # The STAGED blob is scanned, not the diff: content a commit merely carries
    # along is still published by that commit, and a diff-scoped check would
    # wave it through because those lines are not the ones that changed.
    local f i hits=0 tmp
    tmp="$(mktemp)" || return 1
    for f in "${staged[@]}"; do
        git show ":$f" > "$tmp" 2>/dev/null || continue
        for i in "${!pats[@]}"; do
            # -I skips binaries; only the line NUMBER is kept, never the text.
            local nums
            nums="$(grep -I -n -E -- "${pats[$i]}" "$tmp" 2>/dev/null | cut -d: -f1 | tr '\n' ',')"
            if [[ -n "$nums" ]]; then
                printf '  %s: line(s) %s — term #%d\n' "$f" "${nums%,}" "$i" >&2
                hits=$((hits + 1))
            fi
        done
    done
    rm -f "$tmp"

    if [[ $hits -gt 0 ]]; then
        printf '\nERROR pre-commit: staged content matches the restricted-term list.\n' >&2
        printf '  The matched text is deliberately NOT printed — see this gate.\n' >&2
        printf '  Term indexes refer to the non-comment lines of:\n    %s\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  Remove the content and re-stage. SKIP_PRECOMMIT does NOT cover\n' >&2
        printf '  this gate; SCE_RESTRICTED_OVERRIDE=1 does, and says so loudly.\n' >&2
        return 1
    fi
    return 0
}
