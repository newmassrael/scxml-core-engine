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

# ── The loose tier, and the numbers that exclude a pattern from it ─────────
#
# The list is matched twice. The STRICT tier is the list as its owner wrote it:
# case-sensitive, anchors and all, over every staged blob. The LOOSE tier is
# the same list with its word boundaries removed and case ignored, over the
# lines a commit ADDS (and over the whole commit message). It exists for the
# two shapes the strict tier is documented to miss: a term with a letter stuck
# in front of it, and a term in another case. It is built from the list by
# string substitution, so nothing here reads what a pattern says.
#
# A loose pattern is a blunter one, and some patterns are blunt enough to match
# ordinary words: measured 2026-10-17 over 2003 commits, 364 carried a loose hit
# and two patterns produced nearly all of them. So the tier looks at added lines
# only (a file whose old lines happen to match does not refuse a commit that
# did not write them), and a pattern the owner finds too noisy is left out of
# it by NUMBER, in a file beside the list that holds numbers and comments and
# no term. A pattern nobody excluded is checked loosely: a new line in the list
# is covered the day it is written, and a number that is not in the file is
# never a reason to check less.
RESTRICTED_LOOSE_EXCLUDE_FILE="${SCE_RESTRICTED_LOOSE_EXCLUDE:-${RESTRICTED_TERMS_FILE}.loose-exclude}"

# Read the list into RT_PATS (as written), and the loose tier into RT_LOOSE_PATS
# with RT_LOOSE_IDX (each one's number in RT_PATS).
_rt_load() {
    RT_PATS=()
    RT_LOOSE_PATS=()
    RT_LOOSE_IDX=()
    local line loose n=0 left=" "
    if [[ -r "$RESTRICTED_LOOSE_EXCLUDE_FILE" ]]; then
        while IFS= read -r line || [[ -n "$line" ]]; do
            line="${line%%#*}"
            line="${line//[[:space:]]/}"
            [[ "$line" =~ ^[0-9]+$ ]] && left+="$line "
        done < "$RESTRICTED_LOOSE_EXCLUDE_FILE"
    fi
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ -z "${line// /}" ]] && continue
        [[ "$line" == \#* ]] && continue
        RT_PATS+=("$line")
        if [[ "$left" != *" $n "* ]]; then
            loose="${line//\\b/}"
            loose="${loose//\\</}"
            loose="${loose//\\>/}"
            RT_LOOSE_PATS+=("$loose")
            RT_LOOSE_IDX+=("$n")
        fi
        n=$((n + 1))
    done < "$RESTRICTED_TERMS_FILE"
}

# The lines the staged diff adds, as `start count` ranges per path, in
# RT_RANGES. A staged path the diff has no hunk for has an empty range (a mode
# change, a rename, a binary file add no line). A header this parser cannot
# map to a staged path (git quotes a path with a tab or a newline in it) sets
# RT_UNMAPPED, and every line then counts as added: a path it cannot place is
# scanned, never skipped.
_rt_added_ranges() {
    local f
    declare -gA RT_RANGES=()
    RT_UNMAPPED=0
    for f in "$@"; do RT_RANGES["$f"]=""; done
    local rpath rs rc
    while IFS=$'\t' read -r rpath rs rc; do
        if [[ "$rpath" == "?" ]]; then
            RT_UNMAPPED=1
        elif [[ -n "${RT_RANGES[$rpath]+x}" ]]; then
            RT_RANGES["$rpath"]+="$rs $rc;"
        else
            RT_UNMAPPED=1
        fi
    done < <(git -c core.quotePath=false diff --cached -U0 --no-color --no-ext-diff \
                 --src-prefix=a/ --dst-prefix=b/ --diff-filter=ACMR 2>/dev/null \
             | awk '
        /^diff --git / { inhdr = 1; cur = ""; next }
        inhdr && /^\+\+\+ / {
            p = substr($0, 5); sub(/\t.*$/, "", p)
            if (p == "/dev/null") { cur = ""; next }
            if (substr(p, 1, 1) == "\"") { print "?\t0\t0"; cur = ""; next }
            sub(/^b\//, "", p); cur = p; next
        }
        /^@@ / {
            inhdr = 0
            if (cur == "") next
            s = $3; sub(/^\+/, "", s); c = 1
            k = index(s, ",")
            if (k > 0) { c = substr(s, k + 1); s = substr(s, 1, k - 1) }
            print cur "\t" s "\t" c
        }')
}

# The line numbers of $1 ("3,5,") that $2 ("5,") does not already name.
_rt_without() {
    local out="" n
    local IFS=','
    for n in $1; do
        [[ ",$2" == *",$n,"* ]] || out+="$n,"
    done
    printf '%s' "$out"
}

# Whether line $2 of staged path $1 is one the commit adds.
_rt_is_added() {
    [[ "$RT_UNMAPPED" == "1" ]] && return 0
    [[ -z "${RT_RANGES[$1]+x}" ]] && return 0
    local spec="${RT_RANGES[$1]}" r s c
    local IFS=';'
    for r in $spec; do
        s="${r% *}"
        c="${r#* }"
        (( $2 >= s && $2 < s + c )) && return 0
    done
    return 1
}

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

    # ⚠⚠ KNOWN WEAKNESS OF THE STRICT TIER, MEASURED 2026-09-18 AND
    # DELIBERATELY NOT PATCHED IN IT. The loose tier (above, and run below
    # over the lines a commit adds) is the answer to it: the two repairs
    # described here are done there, where they cannot turn an old file red.
    # Several of the list's patterns are anchored `\b<term>`, and a
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
    _rt_load
    local -a pats=("${RT_PATS[@]}")

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
    # The loose tier, over the same scratch root and over the added lines only.
    local -A lfound=()
    local j rel n
    if [[ ${#RT_LOOSE_PATS[@]} -gt 0 ]]; then
        _rt_added_ranges "${staged[@]}"
        for j in "${!RT_LOOSE_PATS[@]}"; do
            while IFS= read -r -d '' path && IFS= read -r hit; do
                rel="${path#"$root"/}"
                n="${hit%%:*}"
                if _rt_is_added "$rel" "$n"; then
                    lfound["${RT_LOOSE_IDX[$j]}:$rel"]+="$n,"
                fi
            done < <(grep -r -I -n -i -Z -E -- "${RT_LOOSE_PATS[$j]}" "$root" 2>/dev/null)
        done
    fi
    rm -rf "$root"

    local hits=0 nums
    for f in "${staged[@]}"; do
        for i in "${!pats[@]}"; do
            nums="${found["$i:$f"]:-}"
            if [[ -n "$nums" ]]; then
                printf '  %s: line(s) %s — term #%d\n' "$f" "${nums%,}" "$i" >&2
                hits=$((hits + 1))
            fi
            nums="${lfound["$i:$f"]:-}"
            # A subshell per file per pattern is what cost seventy minutes once:
            # it is spent only where there is something to subtract from.
            [[ -n "$nums" ]] && nums="$(_rt_without "$nums" "${found["$i:$f"]:-}")"
            if [[ -n "$nums" ]]; then
                printf '  %s: line(s) %s — loose term #%d\n' "$f" "${nums%,}" "$i" >&2
                hits=$((hits + 1))
            fi
        done
    done

    if [[ $hits -gt 0 ]]; then
        printf '\nERROR pre-commit: staged content matches the restricted-term list.\n' >&2
        printf '  The matched text is deliberately NOT printed — see this gate.\n' >&2
        printf '  Term indexes refer to the non-comment lines of:\n    %s\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  "loose term" is the same pattern with case ignored and no word boundary,\n' >&2
        printf '  over the lines this commit adds. A pattern that is too blunt for that is\n' >&2
        printf '  left out by number in:\n    %s\n' "$RESTRICTED_LOOSE_EXCLUDE_FILE" >&2
        printf '  Remove the content and re-stage. SKIP_PRECOMMIT does NOT cover\n' >&2
        printf '  this gate; SCE_RESTRICTED_OVERRIDE=1 does, and says so loudly.\n' >&2
        return 1
    fi
    return 0
}

# The commit message, held to the same list: strict, and loose with every line
# counted as added. `commit-msg` calls it with the message file; lines git
# would drop as comments are not part of the message and are not scanned.
restricted_term_gate_message() {
    local message="${1:?restricted_term_gate_message needs the message file}"
    if [[ "${SCE_RESTRICTED_OVERRIDE:-0}" == "1" ]]; then
        printf '\n⚠ commit-msg: SCE_RESTRICTED_OVERRIDE=1 — restricted-term gate DISABLED.\n' >&2
        printf '  The remote is public and a push does not un-publish. You are\n' >&2
        printf '  asserting that you have read the message yourself.\n\n' >&2
        return 0
    fi
    if [[ ! -r "$RESTRICTED_TERMS_FILE" ]]; then
        printf '\nERROR commit-msg: restricted-term gate cannot read its term list.\n' >&2
        printf '  Expected at: %s\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  This gate FAILS CLOSED, as the one in pre-commit does.\n' >&2
        return 1
    fi
    _rt_load
    if [[ ${#RT_PATS[@]} -eq 0 ]]; then
        printf '\nERROR commit-msg: the restricted-term list is empty (%s).\n' "$RESTRICTED_TERMS_FILE" >&2
        return 1
    fi

    local body
    body="$(mktemp)" || return 1
    # A comment line becomes an empty one, so a line keeps its number.
    sed 's/^#.*$//' "$message" > "$body"

    local i j hit nums strict hits=0
    for i in "${!RT_PATS[@]}"; do
        nums=""
        while IFS= read -r hit; do nums+="${hit%%:*},"; done \
            < <(grep -n -E -- "${RT_PATS[$i]}" "$body" 2>/dev/null)
        if [[ -n "$nums" ]]; then
            printf '  commit message: line(s) %s — term #%d\n' "${nums%,}" "$i" >&2
            hits=$((hits + 1))
        fi
    done
    for j in "${!RT_LOOSE_PATS[@]}"; do
        nums=""
        while IFS= read -r hit; do nums+="${hit%%:*},"; done \
            < <(grep -n -i -E -- "${RT_LOOSE_PATS[$j]}" "$body" 2>/dev/null)
        # What the strict tier named for this pattern is not named twice.
        strict=""
        while IFS= read -r hit; do strict+="${hit%%:*},"; done \
            < <(grep -n -E -- "${RT_PATS[${RT_LOOSE_IDX[$j]}]}" "$body" 2>/dev/null)
        nums="$(_rt_without "$nums" "$strict")"
        if [[ -n "$nums" ]]; then
            printf '  commit message: line(s) %s — loose term #%d\n' "${nums%,}" "${RT_LOOSE_IDX[$j]}" >&2
            hits=$((hits + 1))
        fi
    done
    rm -f "$body"

    if [[ $hits -gt 0 ]]; then
        printf '\nERROR commit-msg: the message matches the restricted-term list.\n' >&2
        printf '  The matched text is deliberately NOT printed — see this gate.\n' >&2
        printf '  Term indexes refer to the non-comment lines of:\n    %s\n' "$RESTRICTED_TERMS_FILE" >&2
        printf '  Rewrite the message. SCE_RESTRICTED_OVERRIDE=1 covers this gate and says so.\n' >&2
        return 1
    fi
    return 0
}
