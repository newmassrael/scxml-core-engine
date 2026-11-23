#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Refuse a push with a commit that changes what a formal check says together with what
# the check judges, or that does so without a reason (docs/adr/0115, D11). Sourced by
# `pre-push` (`spec_isolation_gate_range`, over the commits the push publishes), which
# the landing queue runs too.
#
# The unit is the commit, unlike the consumer guide's: a push that changes a property in
# one commit and the code in the next has shown each by itself. What is a check, and what
# is judged by it, is answered by `spec_isolation.py` beside this file.

# spec_isolation_gate_range <base>..<head>
spec_isolation_gate_range() {
    local range="$1" here found status=0
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    found="$(python3 "$here/spec_isolation.py" "$range")" || status=$?
    (( status == 0 )) && return 0
    if (( status != 1 )); then
        printf '\nERROR spec-isolation: could not judge the range %s\n' "$range" >&2
        return 1
    fi
    printf '\nERROR spec-isolation: a commit changes what a check says (docs/adr/0115, D11):\n' >&2
    while IFS= read -r line; do
        printf '  %s\n' "$line" >&2
    done <<<"$found"
    printf 'Change the check in a commit of its own, with nothing beside it but documents,\n' >&2
    printf 'and name the reason in a bullet of its message:\n  - Spec: <why>\n' >&2
    return 1
}
