#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The refs of a bx LANDING QUEUE, and which checks of the push each of them meets (bx RFC 0024, sections 12 and 4.14).
#
# `bx --land` and its integrator push refs under refs/bx-land/<branch>/ beside the branch they land on. None of them
# moves a branch, so none is a push the gates judge: the integrator judges each candidate by running the pre-push
# hook for the landing push's own line (`refs/heads/main <candidate> refs/heads/main <value>`), and the landing push
# runs it again, which moves main and meets every gate as any push to main does.
#
# ## What each ref meets, and why a name is not enough
#
# Which refs are bx's bookkeeping, which are items, and what each is tested by is bx's description
# (`bx-fleet land refs --describe --tsv`, bx RFC 0024 4.14), read on every push; a kind bx adds needs no change here.
# The bx-fleet asked is the one $BX_FLEET_BIN names (the queue sets it for every push it makes), else the one on PATH;
# never $BX_FLEET, which is bx's fleet given as data and which a bx run under this hook would read as every machine
# unreachable.
# An item meets every check of what a push publishes (terms, identities, the consumer guide) and no gate: its tree is
# judged on its candidate. Bookkeeping meets the term check alone, IF its content is bx's: given to the other checks,
# the identity check would refuse bx's author, the tree check a commit that is not HEAD, and the gate selection,
# finding no base, would run the whole suite.
#
# A ref under refs/bx-land/ that fails its kind's tests is checked as any other push: published AND gated. The NAME
# only ever asks for a test; it never grants an exemption. Otherwise an ordinary commit pushed as `.../log` would
# reach the public remote ungated. So is every such ref when the description cannot be had: the hook fails closed.
#
# ## The identity
#
# bx writes every bookkeeping commit as `bx land <bx-land@localhost>` (bx: crates/bx-fleet/src/land/git.rs, IDENT),
# which its description names. The identity alone is weak (anyone can set it); with "no shared history" and "only the
# kind's files", no commit of this repository can pass.
#
# Everything below is bx's library (tools/git-hooks/bx-land-refs.sh of bx main 308bce89), copied verbatim: replace it
# with the next release's copy rather than editing it here.

BX_LAND_REFS_FORMAT=1

# bx_land_description  -> the kinds' lines on stdout, or 1 with why on stderr.
bx_land_description() {
    local text header
    if [[ -n "${BX_LAND_REFS_DESCRIPTION:-}" ]]; then
        text="$(cat -- "$BX_LAND_REFS_DESCRIPTION" 2>/dev/null)" || { echo "the description $BX_LAND_REFS_DESCRIPTION cannot be read" >&2; return 1; }
    else
        text="$("${BX_FLEET_BIN:-bx-fleet}" land refs --describe --tsv 2>/dev/null)" || { echo "bx-fleet could not describe the queue's refs (is bx installed?)" >&2; return 1; }
    fi
    header="${text%%$'\n'*}"
    if [[ "$header" != "# bx-land-refs $BX_LAND_REFS_FORMAT" ]]; then
        echo "the description is \"$header\", not format $BX_LAND_REFS_FORMAT that this hook reads" >&2
        return 1
    fi
    printf '%s\n' "${text#*$'\n'}"
}

# bx_land_tests <files> <author> <history> <local sha> <remote sha>  -> 0 when every commit the push would publish passes
# the kind's tests, else 1 with the reason on stderr.
bx_land_tests() {
    local files="$1" author="$2" history="$3" tip="$4" old="$5" other c who path
    case "$history" in
        none)
            for other in HEAD refs/remotes/origin/HEAD; do
                git rev-parse --verify --quiet "$other^{commit}" >/dev/null || continue
                if git merge-base "$tip" "$other" >/dev/null 2>&1; then
                    echo "it shares history with $other" >&2
                    return 1
                fi
            done
            ;;
        shares)
            git rev-parse --verify --quiet "refs/remotes/origin/HEAD^{commit}" >/dev/null || { echo "there is no origin/HEAD to compare with" >&2; return 1; }
            git merge-base "$tip" refs/remotes/origin/HEAD >/dev/null 2>&1 || { echo "it shares no history with origin/HEAD" >&2; return 1; }
            ;;
        *)
            echo "the description names a history \"$history\" this hook does not know" >&2
            return 1
            ;;
    esac
    [[ "$files" == "-" && "$author" == "-" ]] && return 0
    local range=("$tip")
    [[ -n "$old" && ! "$old" =~ ^0+$ ]] && git rev-parse --verify --quiet "$old^{commit}" >/dev/null && range+=("^$old")
    while read -r c; do
        [[ -z "$c" ]] && continue
        if [[ "$author" != "-" ]]; then
            who="$(git log -1 --format='%an <%ae>|%cn <%ce>' "$c")"
            if [[ "$who" != "$author|$author" ]]; then
                echo "commit ${c:0:12} is by $who, not $author" >&2
                return 1
            fi
        fi
        if [[ "$files" != "-" ]]; then
            while read -r path; do
                [[ -z "$path" ]] && continue
                if [[ ! "$path" =~ $files ]]; then
                    echo "commit ${c:0:12} holds $path, which its kind does not hold" >&2
                    return 1
                fi
            done < <(git ls-tree -r --name-only "$c")
        fi
    done < <(git rev-list "${range[@]}")
    return 0
}

# bx_land_classify  (the pushed refs on stdin, git's four words a line)  -> sets PUBLISHED_REFS, every ref the checks
# of what a push publishes walk, and GATED_REFS, every ref the gates walk. A deletion is in both (the checks skip it).
bx_land_classify() {
    local lref="" lsha="" rref="" rsha="" line="" why="" kinds="" kind="" refs="" files="" author="" history=""
    local published="" gated="" matched="" undescribed=""
    PUBLISHED_REFS=""
    GATED_REFS=""
    # The failure is the answer here, not an error: its words are kept for the refs it leaves undescribed.
    if ! kinds="$(bx_land_description 2>/dev/null)"; then
        kinds=""
        undescribed="$(bx_land_description 2>&1 >/dev/null)" || true
    fi
    BX_LAND_UNDESCRIBED="$undescribed"
    local entry=""
    # A last line with no newline is a line too (`read` fails on it, having read it).
    while IFS= read -r entry || [[ -n "$entry" ]]; do
        read -r lref lsha rref rsha <<<"$entry" || true
        entry=""
        [[ -n "$lsha" ]] || continue
        line="$lref $lsha $rref $rsha"
        matched=""
        if [[ ! "$lsha" =~ ^0+$ && -n "$kinds" ]]; then
            while IFS=$'\t' read -r kind refs files author history published gated; do
                # A comment, an empty line, or a kind with no ref pattern (which would match every ref) is none.
                [[ -z "$kind" || "$kind" == \#* || -z "$refs" ]] && continue
                [[ "$rref" =~ $refs ]] || continue
                matched="$kind"
                if why="$(bx_land_tests "$files" "$author" "$history" "$lsha" "$rsha" 2>&1)"; then
                    [[ "$published" == 1 ]] && PUBLISHED_REFS+="$line"$'\n'
                    [[ "$gated" == 1 ]] && GATED_REFS+="$line"$'\n'
                    continue 2
                fi
                printf 'pre-push: %s is named as a bx landing queue %s but %s; it is checked as any push\n' "$rref" "$kind" "$why" >&2
                break
            done <<<"$kinds"
        fi
        if [[ -z "$matched" && "$rref" == refs/bx-land/* && ! "$lsha" =~ ^0+$ ]]; then
            if [[ -z "$kinds" ]]; then
                printf 'pre-push: %s is a bx landing queue ref, and its kind cannot be told (%s); it is checked as any push\n' "$rref" "${BX_LAND_UNDESCRIBED:-no description}" >&2
            else
                printf 'pre-push: %s is under refs/bx-land/ but no kind bx describes is; it is checked as any push\n' "$rref" >&2
            fi
        fi
        PUBLISHED_REFS+="$line"$'\n'
        GATED_REFS+="$line"$'\n'
    done
    return 0
}
