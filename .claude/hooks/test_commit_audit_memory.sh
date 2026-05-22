#!/usr/bin/env bash
# Regression tests for commit_audit.sh memory lifecycle contract.
#
# The MEMO_VALIDATION block in commit_audit.sh enforces the contract
# in claudedocs/rfc-memory-sixth-wave.md:
#   - every memory file declares status in 9-value enum
#   - prefix/suffix encodes lifecycle: filename and status must agree
#   - archive bucket invariants: top-level = active, archive/closed/** = closed
#
# Strategy: each test case constructs a synthetic memory dir keyed by a
# unique temp project path (via the hook's CWD → slug → memory dir
# derivation), populates it with one or more fixture .md files, invokes
# the hook with a fake `git commit` payload, and asserts the hook's
# stderr against expected violation messages.
#
# Run: `bash .claude/hooks/test_commit_audit_memory.sh`. Exits 0 on
# success, 1 with details on the first mismatch.

set -euo pipefail

HOOK="$(cd "$(dirname "$0")" && pwd)/commit_audit.sh"
if [ ! -x "$HOOK" ]; then
    echo "FAIL: cannot locate executable commit_audit.sh next to test script" >&2
    exit 1
fi

# Each case builds a unique tmp project so the slug-derived memory dir
# is isolated. Tear down both project and memory dir on exit.
TMP_BASE="$(mktemp -d)"
CREATED_MEMORY_DIRS=()
cleanup() {
    rm -rf "$TMP_BASE"
    for d in "${CREATED_MEMORY_DIRS[@]}"; do
        rm -rf "$d"
    done
}
trap cleanup EXIT

# Build a tmp project + matching memory dir.
# Args: name. Outputs the project path on stdout.
make_project() {
    local name="$1"
    local proj="$TMP_BASE/$name"
    mkdir -p "$proj"
    git -C "$proj" init -q
    local slug
    slug="$(printf '%s' "$proj" | sed 's|/|-|g')"
    local memdir="$HOME/.claude/projects/${slug}/memory"
    mkdir -p "$memdir"
    CREATED_MEMORY_DIRS+=("$HOME/.claude/projects/${slug}")
    printf '%s\n' "$proj"
}

# Write a memory file with given frontmatter status.
# Args: memdir filename status [extra-status-line-override]
write_memo() {
    local memdir="$1" filename="$2" status="$3"
    cat > "$memdir/$filename" <<EOF
---
name: $filename test fixture
description: test fixture for hook regression
status: $status
type: project
---

body
EOF
}

# Write a memory file with NO status field.
write_memo_no_status() {
    local memdir="$1" filename="$2"
    cat > "$memdir/$filename" <<EOF
---
name: $filename test fixture
description: test fixture without status
type: project
---

body
EOF
}

# Invoke the hook with a clean (non-forward-reference) commit message
# in the given project. Capture stderr.
invoke_hook() {
    local proj="$1"
    local payload
    payload="$(python3 -c '
import json, sys
cmd = "git commit -m " + json.dumps("docs: test fixture")
print(json.dumps({"tool_input": {"command": cmd}, "cwd": sys.argv[1]}))
' "$proj")"
    rm -f "$proj/.git/.claude-commit-audit-sha"
    set +e
    printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>"$TMP_BASE/.stderr"
    set -e
    cat "$TMP_BASE/.stderr"
}

# Memory dir for a given project.
memdir_for() {
    local proj="$1"
    local slug
    slug="$(printf '%s' "$proj" | sed 's|/|-|g')"
    printf '%s\n' "$HOME/.claude/projects/${slug}/memory"
}

assert_blocks_with() {
    local desc="$1" needle="$2" stderr="$3"
    if ! grep -q "memory lifecycle contract violations" <<<"$stderr"; then
        echo "FAIL [$desc]: expected memory contract block, got:" >&2
        echo "$stderr" | head -5 | sed 's/^/        /' >&2
        return 1
    fi
    if ! grep -qF "$needle" <<<"$stderr"; then
        echo "FAIL [$desc]: expected violation containing '$needle', got:" >&2
        echo "$stderr" | head -10 | sed 's/^/        /' >&2
        return 1
    fi
    return 0
}

assert_no_memory_block() {
    local desc="$1" stderr="$2"
    if grep -q "memory lifecycle contract violations" <<<"$stderr"; then
        echo "FAIL [$desc]: expected memory contract to PASS, but blocked:" >&2
        echo "$stderr" | head -10 | sed 's/^/        /' >&2
        return 1
    fi
    return 0
}

failures=0

# ── Case 1: missing status field ───────────────────────────────
proj="$(make_project case-missing-status)"
md="$(memdir_for "$proj")"
write_memo_no_status "$md" "next_foo.md"
err="$(invoke_hook "$proj")"
assert_blocks_with "missing-status" "no status field" "$err" || failures=$((failures + 1))

# ── Case 2: invalid status value ───────────────────────────────
proj="$(make_project case-invalid-status)"
md="$(memdir_for "$proj")"
write_memo "$md" "some_landed.md" "in-progress"
err="$(invoke_hook "$proj")"
assert_blocks_with "invalid-status" "invalid status 'in-progress'" "$err" || failures=$((failures + 1))

# ── Case 3: next_*.md with closed status ───────────────────────
proj="$(make_project case-next-closed)"
md="$(memdir_for "$proj")"
write_memo "$md" "next_foo.md" "landed"
err="$(invoke_hook "$proj")"
assert_blocks_with "next-closed" "next_*.md must be status:open" "$err" || failures=$((failures + 1))

# ── Case 4: feedback_*.md with wrong status ────────────────────
proj="$(make_project case-feedback-wrong)"
md="$(memdir_for "$proj")"
write_memo "$md" "feedback_foo.md" "active"
err="$(invoke_hook "$proj")"
assert_blocks_with "feedback-wrong" "feedback_*.md must be status:feedback" "$err" || failures=$((failures + 1))

# ── Case 5: _landed suffix with non-landed status ──────────────
proj="$(make_project case-landed-suffix-mismatch)"
md="$(memdir_for "$proj")"
write_memo "$md" "foo_landed.md" "active"
err="$(invoke_hook "$proj")"
assert_blocks_with "landed-suffix-mismatch" "'_landed' requires status:landed" "$err" || failures=$((failures + 1))

# ── Case 6: _superseded suffix with non-superseded status ──────
proj="$(make_project case-superseded-suffix-mismatch)"
md="$(memdir_for "$proj")"
write_memo "$md" "foo_superseded.md" "landed"
err="$(invoke_hook "$proj")"
assert_blocks_with "superseded-suffix-mismatch" "'_superseded' requires status:superseded" "$err" || failures=$((failures + 1))

# ── Case 7: archive top-level with non-active status ──────────
proj="$(make_project case-archive-toplevel-non-active)"
md="$(memdir_for "$proj")"
mkdir -p "$md/archive"
write_memo "$md/archive" "agg.md" "landed"
err="$(invoke_hook "$proj")"
assert_blocks_with "archive-toplevel-non-active" "archive top-level aggregator must be active" "$err" || failures=$((failures + 1))

# ── Case 8: archive/closed/** with active status ──────────────
proj="$(make_project case-archive-closed-active)"
md="$(memdir_for "$proj")"
mkdir -p "$md/archive/closed/topic"
write_memo "$md/archive/closed/topic" "some_landed.md" "active"
err="$(invoke_hook "$proj")"
assert_blocks_with "archive-closed-active" "archive/closed/** must be a closed status" "$err" || failures=$((failures + 1))

# ── Case 9: clean memory tree passes ──────────────────────────
proj="$(make_project case-clean)"
md="$(memdir_for "$proj")"
write_memo "$md" "next_active_plan.md" "open"
write_memo "$md" "feedback_be_terse.md" "feedback"
write_memo "$md" "topic_landed.md" "landed"
write_memo "$md" "topic_superseded.md" "superseded"
write_memo "$md" "topic_refuted.md" "refuted"
write_memo "$md" "topic_retired.md" "retired"
write_memo "$md" "topic_retrospective.md" "retrospective"
write_memo "$md" "foundational_doc.md" "active"
write_memo "$md" "external_pointer.md" "reference"
mkdir -p "$md/archive/closed/topic"
write_memo "$md/archive" "aggregator.md" "active"
write_memo "$md/archive/closed/topic" "moved_landed.md" "landed"
err="$(invoke_hook "$proj")"
assert_no_memory_block "clean-tree-passes" "$err" || failures=$((failures + 1))

# ── Case 10: _done alias → status:landed ───────────────────────
proj="$(make_project case-done-alias-ok)"
md="$(memdir_for "$proj")"
write_memo "$md" "session_foo_done.md" "landed"
err="$(invoke_hook "$proj")"
assert_no_memory_block "done-alias-landed-ok" "$err" || failures=$((failures + 1))

# ── Case 11: _absorbed alias → status:superseded ───────────────
proj="$(make_project case-absorbed-alias-ok)"
md="$(memdir_for "$proj")"
write_memo "$md" "session_foo_absorbed.md" "superseded"
err="$(invoke_hook "$proj")"
assert_no_memory_block "absorbed-alias-superseded-ok" "$err" || failures=$((failures + 1))

if [ "$failures" -gt 0 ]; then
    echo "" >&2
    echo "$failures memory-lifecycle test case(s) failed." >&2
    exit 1
fi

echo "OK: 11 memory lifecycle contract cases verified against live hook."
