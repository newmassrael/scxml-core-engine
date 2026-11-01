#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Refuse a push whose commits change what a downstream consumer compiles against,
# or what a decode refuses, without saying how to follow it (docs/adr/0078 §2).
# Sourced by `pre-push` (`consumer_guide_gate_range`, over the commits the push
# publishes).
#
# The unit is the push, not the commit: a landing here is a `feat` commit and the
# `chore` commit that regenerates its goldens, and the guide section belongs to the
# `feat`. Judged commit by commit, the `chore` would be refused for a section its
# landing already has.
#
# What is consumer-visible is answered by `consumer_visible_change.py` beside this
# file. A range it names a path for must also change
# `docs/SCE_CODEC_CONSUMER_MIGRATION.md`, or carry, in one of its commit messages,
# a bullet that starts `- No consumer-visible change:` and says why. The bullet is
# the author's claim and the reviewer's to check; the gate only makes it written.

CONSUMER_GUIDE="docs/SCE_CODEC_CONSUMER_MIGRATION.md"
CONSUMER_GUIDE_WAIVER='^- No consumer-visible change: .+'

# consumer_guide_gate_range <base>..<head>
consumer_guide_gate_range() {
    local range="$1" here visible path
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if ! visible="$(python3 "$here/consumer_visible_change.py" "$range")"; then
        printf '\nERROR consumer-guide: could not read the range %s\n' "$range" >&2
        return 1
    fi
    [[ -z "$visible" ]] && return 0
    if [[ -n "$(git diff --name-only "$range" -- "$CONSUMER_GUIDE")" ]]; then
        return 0
    fi
    if git log --format=%B "$range" | grep -Eq "$CONSUMER_GUIDE_WAIVER"; then
        return 0
    fi
    printf '\nERROR consumer-guide: these commits change what a downstream consumer\n' >&2
    printf 'compiles against, or what a decode refuses, and %s\n' "$CONSUMER_GUIDE" >&2
    printf 'has no section for it (docs/adr/0078 §2):\n' >&2
    while IFS= read -r path; do
        printf '  %s\n' "$path" >&2
    done <<<"$visible"
    printf 'Add the section (commits, what breaks, how to write it now), or, if nothing\n' >&2
    printf 'a consumer uses changed, say so in a commit message with a bullet\n' >&2
    printf '  - No consumer-visible change: <why>\n' >&2
    return 1
}
