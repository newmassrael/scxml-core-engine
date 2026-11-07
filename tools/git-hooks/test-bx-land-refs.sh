#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The checks a bx landing queue's refs meet in pre-push (bx-land-refs.sh), on a repository made here: a ref's NAME
# under refs/bx-land/ never exempts it; only content that passes its arm's test narrows what it meets.
#
#   1. a genuine record (bx's identity, unrelated history, the record's files) meets the term check alone
#   2. this repository's commit pushed as .../log is published AND gated
#   3. bx's identity and unrelated history, but a source file in the tree: published and gated
#   4. unrelated history and the record's files, but a person's identity: published and gated
#   5. a genuine lease meets the term check alone
#   6. an item sharing history with origin/HEAD is published and not gated
#   7. an item sharing no history with it is published and gated
#   8. a push to main is published and gated, as before
#
# Usage: tools/git-hooks/test-bx-land-refs.sh   (no network, no build; a few seconds)

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tools/git-hooks/bx-land-refs.sh
source "$HERE/bx-land-refs.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
git init -q -b main repo
cd repo

person() { git -c user.name=dev -c user.email=dev@example.invalid "$@"; }
bxid() {
    GIT_AUTHOR_NAME="bx land" GIT_AUTHOR_EMAIL=bx-land@localhost \
    GIT_COMMITTER_NAME="bx land" GIT_COMMITTER_EMAIL=bx-land@localhost git "$@"
}
blob() { printf '%s\n' "$1" | git hash-object -w --stdin; }
# orphan <who: bxid|person> <name=text>...  -> a commit with no parent holding those top-level files
orphan() {
    local who="$1" lines="" path text tree
    shift
    for kv in "$@"; do
        path="${kv%%=*}"
        text="${kv#*=}"
        lines+="100644 blob $(blob "$text")	$path"$'\n'
    done
    tree="$(printf '%s' "$lines" | git mktree --missing)"
    "$who" commit-tree "$tree" -m orphan
}

# This repository: a commit on main, which origin/HEAD names.
mkdir -p src
printf 'int main() {}\n' > src/main.cpp
git add src/main.cpp
person commit -qm seed
git update-ref refs/remotes/origin/main HEAD
git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
printf 'int main() { return 0; }\n' > src/main.cpp
git add src/main.cpp
person commit -qm change
HEAD_SHA="$(git rev-parse HEAD)"

# A record: queue.toml, next and an item file; a second record commit on it.
items_dir="$(git mktree --missing < <(printf '100644 blob %s\t0000000001-ab12cd34ef56.toml\n' "$(blob 'seq = 1')"))"
rec_tree="$(printf '100644 blob %s\tnext\n100644 blob %s\tqueue.toml\n040000 tree %s\titems\n' \
    "$(blob 2)" "$(blob 'format = 1')" "$items_dir" | git mktree --missing)"
REC1="$(bxid commit-tree "$rec_tree" -m 'submit ab12cd34ef56 0123456789ab')"
REC2="$(bxid commit-tree "$rec_tree" -p "$REC1" -m 'cancel 1 by m')"
LEASE="$(orphan bxid 'holder.toml=token = "aa"')"
FOREIGN_FILE="$(orphan bxid 'next=2' 'evil.cpp=int x;')"
PERSON_RECORD="$(orphan person 'next=2' 'queue.toml=format = 1')"
UNRELATED_ITEM="$(orphan person 'other.cpp=int y;')"
Z=0000000000000000000000000000000000000000

fails=0
# case <name> <published?> <gated?> <one pushed line>
case_() {
    local name="$1" want_pub="$2" want_gated="$3" line="$4" pub gated
    bx_land_classify <<<"$line" 2>/dev/null
    pub=no; gated=no
    [[ -n "$PUBLISHED_REFS" ]] && pub=yes
    [[ -n "$GATED_REFS" ]] && gated=yes
    if [[ "$pub $gated" == "$want_pub $want_gated" ]]; then
        printf 'PASS %s\n' "$name"
    else
        printf 'FAIL %s: published=%s gated=%s, want %s %s\n' "$name" "$pub" "$gated" "$want_pub" "$want_gated"
        fails=$((fails + 1))
    fi
}

case_ "1 a genuine record meets the term check alone" no no "$REC2 $REC2 refs/bx-land/main/log $REC1"
case_ "2 this repository's commit pushed as .../log is published and gated" yes yes "$HEAD_SHA $HEAD_SHA refs/bx-land/main/log $Z"
case_ "3 bx's identity with a source file in the tree is published and gated" yes yes "$FOREIGN_FILE $FOREIGN_FILE refs/bx-land/main/log $Z"
case_ "4 the record's files by a person are published and gated" yes yes "$PERSON_RECORD $PERSON_RECORD refs/bx-land/main/log $Z"
case_ "5 a genuine lease meets the term check alone" no no "$LEASE $LEASE refs/bx-land/main/lease $Z"
case_ "6 an item sharing history with origin/HEAD is published, not gated" yes no "$HEAD_SHA $HEAD_SHA refs/bx-land/main/items/ab12cd34ef56 $Z"
case_ "7 an item sharing no history with it is published and gated" yes yes "$UNRELATED_ITEM $UNRELATED_ITEM refs/bx-land/main/items/cd34ef56ab12 $Z"
case_ "8 a push to main is published and gated" yes yes "refs/heads/main $HEAD_SHA refs/heads/main $Z"
exit "$fails"
