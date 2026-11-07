#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The refs of a bx LANDING QUEUE, and which checks of the push each of them meets (bx RFC 0024, section 12).
#
# `bx --land` and its integrator push refs under refs/bx-land/<branch>/ beside the branch they land on. None of them
# moves a branch, so none is a push the gates judge: the integrator judges each candidate by running the pre-push
# hook for the landing push's own line (`refs/heads/main <candidate> refs/heads/main <value>`), and the landing push
# runs it again, which moves main and meets every gate as any push to main does.
#
# ## What each ref meets, and why a name is not enough
#
#   .../log, .../lease  The queue's record and its integrators' lease: commits of bx's own bookkeeping. They meet the
#                       term check alone, IF their content is bx's bookkeeping: every commit pushed shares no history
#                       with HEAD or the remote's default branch, is authored and committed by bx's fixed identity,
#                       and holds only the record's files (or only the lease's). Given to the other checks, the
#                       identity check would refuse bx's author, the tree check a commit that is not HEAD, and the
#                       gate selection, finding no base, would run the whole suite.
#   .../items/<id>      A submitted item: this repository's commit, published before it lands. It meets every check
#                       of what a push publishes (terms, identities, the consumer guide) and no gate: its tree is
#                       judged on its candidate. It must share history with the remote's default branch. It need not
#                       descend from that branch's tip: the branch moves on after the item is submitted, and the
#                       integrator puts the item on the branch's value when its turn comes.
#
# A ref under refs/bx-land/ that fails its arm's test is checked as any other push: published AND gated. The NAME
# only ever asks for a test; it never grants an exemption. Otherwise an ordinary commit pushed as `.../log` would
# reach the public remote ungated.
#
# ## The identity
#
# bx writes every record and lease commit as `bx land <bx-land@localhost>` (bx: crates/bx-fleet/src/land/git.rs,
# IDENT). The identity alone is weak (anyone can set it); with "no shared history" and "only the record's files",
# no commit of this repository can pass.

BX_LAND_IDENT="bx land <bx-land@localhost>"

# bx_land_bookkeeping <kind> <local sha> <remote sha>  -> 0 when every commit the push would publish under a
# `log` (kind=log) or `lease` (kind=lease) ref is bx's bookkeeping (see above), else 1 with the reason on stderr.
bx_land_bookkeeping() {
    local kind="$1" tip="$2" old="$3" other c who paths path pattern
    for other in HEAD refs/remotes/origin/HEAD; do
        git rev-parse --verify --quiet "$other^{commit}" >/dev/null || continue
        if git merge-base "$tip" "$other" >/dev/null 2>&1; then
            echo "it shares history with $other" >&2
            return 1
        fi
    done
    case "$kind" in
        log) pattern='^(queue\.toml|next|items/[0-9]{10}-[a-z0-9]+\.toml)$' ;;
        lease) pattern='^holder\.toml$' ;;
        *) return 1 ;;
    esac
    local range=("$tip")
    [[ -n "$old" && ! "$old" =~ ^0+$ ]] && git rev-parse --verify --quiet "$old^{commit}" >/dev/null && range+=("^$old")
    while read -r c; do
        [[ -z "$c" ]] && continue
        who="$(git log -1 --format='%an <%ae>|%cn <%ce>' "$c")"
        if [[ "$who" != "$BX_LAND_IDENT|$BX_LAND_IDENT" ]]; then
            echo "commit ${c:0:12} is by $who, not bx's own identity" >&2
            return 1
        fi
        paths="$(git ls-tree -r --name-only "$c")"
        while read -r path; do
            [[ -z "$path" ]] && continue
            if [[ ! "$path" =~ $pattern ]]; then
                echo "commit ${c:0:12} holds $path, which is not a file of the queue's $kind" >&2
                return 1
            fi
        done <<<"$paths"
    done < <(git rev-list "${range[@]}")
    return 0
}

# bx_land_item <local sha>  -> 0 when an item ref's commit shares history with the remote's default branch (else 1).
bx_land_item() {
    git rev-parse --verify --quiet "refs/remotes/origin/HEAD^{commit}" >/dev/null || { echo "there is no origin/HEAD to compare with" >&2; return 1; }
    git merge-base "$1" refs/remotes/origin/HEAD >/dev/null 2>&1 || { echo "it shares no history with origin/HEAD" >&2; return 1; }
}

# bx_land_classify  (the pushed refs on stdin, git's four words a line)  -> sets PUBLISHED_REFS, every ref but a
# genuine record or lease, and GATED_REFS, every ref but those and genuine items: the lists the checks of a push walk.
# A deletion is published (the checks skip it) and gated alike. What a test refused is said on stderr.
bx_land_classify() {
    local lref lsha rref rsha line why kind
    PUBLISHED_REFS=""
    GATED_REFS=""
    while read -r lref lsha rref rsha; do
        [[ -z "${lsha:-}" ]] && continue
        line="$lref $lsha $rref $rsha"
        if [[ ! "$lsha" =~ ^0+$ ]]; then
            if [[ "$rref" =~ ^refs/bx-land/.+/(log|lease)$ ]]; then
                kind="${BASH_REMATCH[1]}"
                if why="$(bx_land_bookkeeping "$kind" "$lsha" "$rsha" 2>&1)"; then
                    continue
                fi
                printf 'pre-push: %s is named as a bx landing queue %s but is not bx'"'"'s bookkeeping (%s); it is checked as any push\n' \
                    "$rref" "$kind" "$why" >&2
            elif [[ "$rref" =~ ^refs/bx-land/.+/items/[^/]+$ ]]; then
                if why="$(bx_land_item "$lsha" 2>&1)"; then
                    PUBLISHED_REFS+="$line"$'\n'
                    continue
                fi
                printf 'pre-push: %s is named as a bx landing queue item but %s; it is checked as any push\n' "$rref" "$why" >&2
            fi
        fi
        PUBLISHED_REFS+="$line"$'\n'
        GATED_REFS+="$line"$'\n'
    done
}
