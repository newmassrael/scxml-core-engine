#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

    # `-z`, because without it git QUOTES a path that is not plain ASCII, and a
    # quoted path names no index entry. The list used to be read without it,
    # so such a file was skipped and its content never scanned — measured
    # 2026-09-28 by `the_restricted_term_gate_runs_each_pattern_once`.
    local -a staged=()
    mapfile -d '' -t staged < <(git diff --cached --name-only -z --diff-filter=ACMR)
    [[ ${#staged[@]} -eq 0 ]] && return 0

    # The STAGED blob is scanned, not the diff: content a commit merely carries
    # along is still published by that commit, and a diff-scoped check would
    # wave it through because those lines are not the ones that changed.
    #
    # Every staged blob is written ONCE under a scratch root, and each pattern
    # is then ONE recursive grep over it. The earlier form ran a grep per file
    # per pattern inside a command substitution, and a commit of 3,509 files
    # spent about seventy minutes here on 2026-09-28. The cost now follows the
    # pattern count; the file count only sizes one checkout.
    #
    # `checkout-index` writes the index version of each path. Two settings keep
    # those bytes equal to `git show :<path>`: `core.symlinks=false` writes a
    # symlink as a file holding its target text, which is what the commit
    # publishes, and `core.autocrlf=false` turns line-ending conversion off. A
    # path whose attributes still ask for a conversion (a filter, `text`,
    # `eol`) is rewritten from the raw blob, so no attribute can change what is
    # scanned.
    local root
    root="$(mktemp -d)" || return 1
    printf '%s\0' "${staged[@]}" \
        | git -c core.symlinks=false -c core.autocrlf=false \
              checkout-index -z --stdin --force --prefix="$root/" 2>/dev/null
    local path attr value
    while IFS= read -r -d '' path && IFS= read -r -d '' attr && IFS= read -r -d '' value; do
        [[ "$value" == "unspecified" || "$value" == "unset" ]] && continue
        git cat-file blob ":$path" > "$root/$path" 2>/dev/null || rm -f "$root/$path"
    done < <(printf '%s\0' "${staged[@]}" | git check-attr -z --cached --stdin filter text eol)

    # A path the checkout did not write is written from its blob here, so a
    # blob can never drop out of the scan in silence. What still fails is not
    # a blob (a gitlink), which the earlier form skipped the same way.
    local f
    for f in "${staged[@]}"; do
        [[ -e "$root/$f" ]] && continue
        mkdir -p "$(dirname "$root/$f")"
        git cat-file blob ":$f" > "$root/$f" 2>/dev/null || rm -f "$root/$f"
    done

    # -I skips binaries; -Z ends each file name with NUL, so no character in a
    # path can be mistaken for the separator; only the line NUMBER is kept,
    # never the text.
    local -A found=()
    local i hit
    for i in "${!pats[@]}"; do
        while IFS= read -r -d '' path && IFS= read -r hit; do
            found["$i:${path#"$root"/}"]+="${hit%%:*},"
        done < <(grep -r -I -n -Z -E -- "${pats[$i]}" "$root" 2>/dev/null)
    done
    rm -rf "$root"

    local hits=0 nums
    for f in "${staged[@]}"; do
        for i in "${!pats[@]}"; do
            nums="${found["$i:$f"]:-}"
            if [[ -n "$nums" ]]; then
                printf '  %s: line(s) %s — term #%d\n' "$f" "${nums%,}" "$i" >&2
                hits=$((hits + 1))
            fi
        done
    done

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
