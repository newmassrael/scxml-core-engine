#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regression tests for the trailer paragraph of commit_audit.sh (COMMIT_FORMAT.md "Trailers", bx RFC 0087): a
# message ending in a blank line and Change-Id / Supersedes lines is judged on the body before them; a trailer
# anywhere else is refused by name. Runs the live hook on a mocked PreToolUse payload in a throwaway repository,
# as test_commit_audit.sh does, and reads its stderr: the format banner, and the violation's tag.
#
# Run: `bash .claude/hooks/test_commit_audit_trailers.sh`. Exits 0 when every case holds.
set -euo pipefail

HOOK="$(cd "$(dirname "$0")" && pwd)/commit_audit.sh"
[ -x "$HOOK" ] || { echo "FAIL: cannot locate executable commit_audit.sh next to this test" >&2; exit 1; }
TMP_REPO="$(mktemp -d)"
trap 'rm -rf "$TMP_REPO"' EXIT
git -C "$TMP_REPO" init -q

stderr_for() {
    local payload
    payload="$(python3 -c '
import json, shlex, sys
print(json.dumps({"tool_input": {"command": "git commit -m " + shlex.quote(sys.argv[1])}, "cwd": sys.argv[2]}))
' "$1" "$TMP_REPO")"
    rm -f "$TMP_REPO/.git/.claude-commit-audit-sha"
    set +e
    printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>"$TMP_REPO/.stderr"
    set -e
    cat "$TMP_REPO/.stderr"
}

ID="I0123456789abcdef0123456789abcdef01234567"
NL=$'\n'
failures=0
expect_clean() { # <label> <message>
    if grep -q "COMMIT_FORMAT.md violation" <<< "$(stderr_for "$2")"; then
        echo "FAIL [$1]: a format violation was reported" >&2; failures=$((failures + 1))
    fi
}
expect_tag() { # <label> <tag> <message>
    if ! grep -qF "[$2]" <<< "$(stderr_for "$3")"; then
        echo "FAIL [$1]: no [$2]" >&2; failures=$((failures + 1))
    fi
}
S="fix: a parser keeps its state"
B="- the state survives a reset"
expect_clean "bullets then a Change-Id" "$S$NL$NL$B$NL$NL""Change-Id: $ID"
expect_clean "a subject alone then a Change-Id" "$S$NL$NL""Change-Id: $ID"
expect_clean "a Supersedes with the Change-Id" "$S$NL$NL$B$NL$NL""Supersedes: $ID$NL""Change-Id: $ID"
expect_tag "a Supersedes above the bullets" trailer-misplaced "$S$NL$NL""Supersedes: $ID$NL$NL$B"
expect_tag "a Change-Id with no blank line before it" trailer-misplaced "$S$NL$NL$B$NL""Change-Id: $ID"

if [ "$failures" -gt 0 ]; then echo "$failures trailer case(s) failed." >&2; exit 1; fi
echo "OK: 5 trailer cases verified against the live hook."
