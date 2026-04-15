#!/usr/bin/env bash
# Regression tests for commit_audit.sh forward-reference detector.
#
# The patterns in commit_audit.sh's COMMIT_MSG_DEFERRAL block guard
# against a specific failure mode (the L9 self-deception path:
# "textbook" claim plus hand-wave reference to later work). Without
# this regression file, a future edit to commit_audit.sh that
# accidentally narrows or breaks a pattern would silently let the
# deferral phrasing slip through — which is exactly what the audit
# was designed to prevent.
#
# Strategy: invoke the live hook with a mocked PreToolUse JSON
# payload and check the exit code. The hook exits 2 on a hard-block
# (forward reference present) and writes a marker on the audit-only
# path (clean message). We test both outcomes — no in-test pattern
# duplication that could drift from the source of truth.
#
# Run: `bash .claude/hooks/test_commit_audit.sh`. Exits 0 on success,
# 1 with details on the first mismatch.

set -euo pipefail

HOOK="$(cd "$(dirname "$0")" && pwd)/commit_audit.sh"
if [ ! -x "$HOOK" ]; then
    echo "FAIL: cannot locate executable commit_audit.sh next to test script" >&2
    exit 1
fi

# Run the hook against a fake commit command. We do NOT want the hook
# to actually pass on the audit-only path (it would write a marker into
# the real .git directory and pollute state for the next real commit).
# Solution: run inside a throwaway git repo so the marker writes there.
TMP_REPO="$(mktemp -d)"
trap 'rm -rf "$TMP_REPO"' EXIT
git -C "$TMP_REPO" init -q

# Reset the marker before each invocation so a prior matched marker
# cannot mask a regression on the next case.
clear_marker() {
    rm -f "$TMP_REPO/.git/.claude-commit-audit-sha"
}

# Build a PreToolUse JSON payload mimicking what Claude Code feeds the
# hook. The hook reads `.tool_input.command` and `.cwd` via the
# `read_field` helper at the top of the script.
build_payload() {
    local commit_msg="$1"
    python3 -c '
import json, sys
msg = sys.argv[1]
cwd = sys.argv[2]
cmd = "git commit -m " + json.dumps(msg)
print(json.dumps({
    "tool_input": {"command": cmd},
    "cwd": cwd,
}))
' "$commit_msg" "$TMP_REPO"
}

# Returns the hook exit code without invoking the actual git commit.
run_hook_for_message() {
    local commit_msg="$1"
    clear_marker
    local payload
    payload="$(build_payload "$commit_msg")"
    # Hook exits 2 on hard-block, exits with audit-block (writes marker)
    # in the audit path. We capture exit code and discard stderr — only
    # the numeric outcome matters for the regression check.
    set +e
    printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>&1
    local rc=$?
    set -e
    echo "$rc"
}

# Each row: expected outcome (BLOCK / ALLOW), description, message
# - BLOCK (rc=2): forward-reference detector must fire
# - ALLOW: forward-reference detector must NOT fire (hook may still
#   block via the audit-mandate path with a different rc, but the
#   detector should let the message through to the audit path —
#   distinguishable by clearing the marker and observing rc=2 with
#   different stderr content; for this regression test we only check
#   that BLOCK cases all hit rc=2 and ALLOW cases hit rc=2 *for a
#   different reason* (the audit prompt). Both rc match — so we
#   instead pin on stderr keywords.
CASES=(
    # BLOCK rows — hit one of the FWD_PATTERNS in commit_audit.sh
    "BLOCK|to-be-wired|to be wired in the next session"
    "BLOCK|consumer-follows|consumer follows in next commit"
    "BLOCK|next-session|next session will add tests"
    "BLOCK|next-commit|deferred to next commit"
    "BLOCK|to-be-implemented|to be implemented in commit X"
    "BLOCK|will-be-landed|the cleanup will be landed in F5"
    "BLOCK|will-follow|tests will follow"
    "BLOCK|upcoming-commit|upcoming commit will fix this"
    "BLOCK|consumer-lands|consumer lands later"
    "BLOCK|placeholder|placeholder for the real impl"
    "BLOCK|TBD|TBD: write the real test"
    "BLOCK|tracked-follow-up|tracked follow-up restores coverage"
    # ALLOW rows — none of the patterns match
    "ALLOW|legit deferred-to phrasing|deferred to runtime evaluation"
    "ALLOW|plain fix|fix parser: handle empty datamodel correctly"
    "ALLOW|spec reference|implements section 9.5 wire mapping"
    "ALLOW|past-tense wired|wired the callback in the ctor"
    "ALLOW|backward stage marker|already landed in F3a"
    "ALLOW|generic followup word|cleans up follow-up notes from the doc"
)

# More precise check: capture stderr to distinguish "blocked by
# forward-reference detector" (BLOCK case) from "blocked by audit
# mandate" (ALLOW case — clean message still triggers the audit
# prompt the first time). Both produce rc=2, so the differentiator
# is the banner string in stderr.
run_hook_capture_stderr() {
    local commit_msg="$1"
    clear_marker
    local payload
    payload="$(build_payload "$commit_msg")"
    set +e
    printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>"$TMP_REPO/.stderr"
    set -e
    cat "$TMP_REPO/.stderr"
}

failures=0
for row in "${CASES[@]}"; do
    expected=${row%%|*}
    rest=${row#*|}
    desc=${rest%%|*}
    msg=${rest#*|}
    err="$(run_hook_capture_stderr "$msg")"
    case "$expected" in
        BLOCK)
            if ! grep -q "forward-reference in commit body" <<<"$err"; then
                echo "FAIL [$desc]: expected forward-reference block but got:" >&2
                echo "$err" | head -3 | sed 's/^/        /' >&2
                failures=$((failures + 1))
            fi
            ;;
        ALLOW)
            if grep -q "forward-reference in commit body" <<<"$err"; then
                echo "FAIL [$desc]: expected detector to allow but it blocked: $msg" >&2
                failures=$((failures + 1))
            fi
            ;;
        *)
            echo "FAIL [$desc]: malformed CASES row, expected BLOCK/ALLOW got '$expected'" >&2
            failures=$((failures + 1))
            ;;
    esac
done

if [ "$failures" -gt 0 ]; then
    echo "" >&2
    echo "$failures forward-reference pattern test(s) failed." >&2
    echo "Either fix commit_audit.sh patterns or update CASES in this test." >&2
    exit 1
fi

echo "OK: ${#CASES[@]} forward-reference pattern cases verified against live hook."
