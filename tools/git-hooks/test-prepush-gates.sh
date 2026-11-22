#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# What the pre-push hook hands the gates (prepush-gates.sh), on a repository made here, with a stand-in
# bx and a stand-in runner that record what they were asked and end with a status the case chooses.
#
# The selection:
#   1. an update to refs/heads/main asks for the landing selection (--landing)
#   2. a push of another branch does not
#   3. a deletion of main publishes nothing and does not
#   4. a branch whose name only starts with main does not
#   5. a push of a branch and main together does
#
# The whole hook, run as git runs it, so the wiring is what is tested and not a copy of it:
#   6. a main update through bx plans the landing selection of its range
#   7. bx's 2 (a unit not measured) is the hook's 2, never a 0 that would land the item
#   8. bx's 1 is the hook's 1, and its 0 the hook's 0
#   9. a branch push through bx plans without --landing
#  10. in place (no bx), a main update hands the runner --landing, and the runner's 3 is the hook's 3
#  11. in place, a branch push's runner status 2 is the hook's 2, and no --landing
#  12. PREPUSH_FULL=1 on a main update plans every gate the fleet can run (--all --landing), status as is
#  13. a genuine landing queue item (refs/bx-land/main/items/<id>) is published and asks for no gate
#
# The landing queue's refs are classified by bx's description of their kinds (bx-land-refs.sh). The
# fixture states that description as a file, as test-bx-land-refs.sh does, so no case asks a bx-fleet
# this machine may carry, and refs/heads/main is gated whichever way the description reads.
#
# A landing counts no skip as a pass, in the gate library:
#  14-18. a missing tool, or a skip of the gate, is 3 in a landing and the skip it was outside one
#  19-21. a suite case that finds its tool missing is 3 in a landing; a suite's own failure is 1
#  22. the runner exports SCE_GATE_LANDING for --landing
#  23. in a landing, a suite that passes beside a service the gate started returns without waiting for it
#
# Usage: tools/git-hooks/test-prepush-gates.sh   (no network, no build; a few seconds)

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tools/git-hooks/prepush-gates.sh
source "$HERE/prepush-gates.sh"
# shellcheck source=tools/git-hooks/ident-gate.sh
source "$HERE/ident-gate.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
Z=0000000000000000000000000000000000000000
A=1111111111111111111111111111111111111111
B=2222222222222222222222222222222222222222

fails=0
pass() { printf 'PASS %s\n' "$1"; }
fail() { printf 'FAIL %s: %s\n' "$1" "$2"; fails=$((fails + 1)); }

# select_case <name> <want: landing|none> <pushed lines>
select_case() {
    local name="$1" want="$2" lines="$3" got=none
    prepush_select_refs <<<"$lines"
    (( ${#PREPUSH_SELECT_ARGS[@]} )) && got="${PREPUSH_SELECT_ARGS[*]}"
    [[ "$got" == "--landing" ]] && got=landing
    if [[ "$got" == "$want" ]]; then pass "$name"; else fail "$name" "selection '$got', want '$want'"; fi
}

select_case "1 an update to main asks for the landing selection" landing "refs/heads/main $A refs/heads/main $B"
select_case "2 a push of another branch does not" none "refs/heads/topic $A refs/heads/topic $B"
select_case "3 a deletion of main does not" none "(delete) $Z refs/heads/main $B"
select_case "4 a branch named mainline does not" none "refs/heads/mainline $A refs/heads/mainline $B"
select_case "5 a branch and main together do" landing "refs/heads/topic $A refs/heads/topic $B
refs/heads/main $A refs/heads/main $B"

# ── The whole hook, on a repository made here ────────────────────────────────────────────────────
#
# The hook files are copied in, since the hook sources its neighbours by its own path, and
# scripts/gate is a stand-in. Commits are made under the first identity ident-gate.sh accepts, and
# the restricted-term list is one term no fixture holds, so the checks ahead of the gates pass.
REPO="$WORK/repo"
LOG="$WORK/asked"
mkdir -p "$REPO/tools/git-hooks" "$REPO/scripts" "$WORK/bin"
cp "$HERE"/* "$REPO/tools/git-hooks/"
cat >"$REPO/scripts/gate" <<'STUB'
#!/usr/bin/env bash
printf 'runner %s\n' "$*" >>"$PREPUSH_TEST_LOG"
exit "${PREPUSH_TEST_RUNNER_RC:-0}"
STUB
chmod +x "$REPO/scripts/gate"
cat >"$WORK/bin/bx" <<'STUB'
#!/usr/bin/env bash
printf 'bx %s\n' "$*" >>"$PREPUSH_TEST_LOG"
exit "${PREPUSH_TEST_BX_RC:-0}"
STUB
chmod +x "$WORK/bin/bx"
printf 'zz-fixture-term-no-file-here-holds-qq\n' >"$WORK/terms.txt"
# bx's description of the queue's kinds, the item kind as bx describes it (format 1). An item shares
# history with origin/HEAD, is published, and is not gated.
{
    printf '# bx-land-refs 1\n'
    printf 'item\t^refs/bx-land/.+/items/[^/]+$\t-\t-\tshares\t1\t0\n'
} >"$WORK/description.tsv"

EMAIL="${SCE_ALLOWED_IDENT_EMAILS[0]}"
fixture_git() { git -C "$REPO" -c user.name=fixture -c "user.email=$EMAIL" "$@"; }
fixture_git init -q -b main
fixture_git add -A
fixture_git commit -qm "seed"
BASE="$(fixture_git rev-parse HEAD)"
# The remote's main is the seed, as it is when the integrator judges a candidate on top of it.
fixture_git update-ref refs/remotes/origin/main "$BASE"
fixture_git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
printf 'a change\n' >"$REPO/notes.txt"
fixture_git add notes.txt
fixture_git commit -qm "change"
TIP="$(fixture_git rev-parse HEAD)"

# hook_case <name> <want status> <want asked line> <pushed line> [VAR=value...]
#
# Runs the hook from the fixture with the line on stdin and the variables set, and compares its exit
# status and the one line the stand-ins recorded.
hook_case() {
    local name="$1" want_rc="$2" want_asked="$3" line="$4" rc=0 asked
    shift 4
    : >"$LOG"
    (
        cd "$REPO"
        env -u BX -u PREPUSH_FULL -u SKIP_PREPUSH -u BX_FLEET_BIN \
            BX_LAND_REFS_DESCRIPTION="$WORK/description.tsv" \
            PREPUSH_TEST_LOG="$LOG" \
            SCE_RESTRICTED_TERMS="$WORK/terms.txt" \
            SCE_RESTRICTED_OVERRIDE_LOG="$WORK/override.log" \
            "$@" bash tools/git-hooks/pre-push origin https://example.invalid/repo.git <<<"$line"
    ) >"$WORK/hook.out" 2>&1 || rc=$?
    asked="$(cat "$LOG")"
    if [[ "$rc" != "$want_rc" ]]; then
        fail "$name" "the hook exited $rc, want $want_rc; its output:"
        sed 's/^/      /' "$WORK/hook.out"
    elif [[ "$asked" != "$want_asked" ]]; then
        fail "$name" "the stand-ins were asked '$asked', want '$want_asked'"
    else
        pass "$name"
    fi
}

MAIN="refs/heads/main $TIP refs/heads/main $BASE"
TOPIC="refs/heads/topic $TIP refs/heads/topic $BASE"
PLAN="bx --checks prepush -- bash scripts/gate --plan"

hook_case "6 a main update through bx plans the landing selection" 0 \
    "$PLAN --landing --changed-in $BASE..$TIP" "$MAIN" BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=0
hook_case "7 bx's 2, a unit not measured, is the hook's 2" 2 \
    "$PLAN --landing --changed-in $BASE..$TIP" "$MAIN" BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=2
hook_case "8 bx's 1 is the hook's 1" 1 \
    "$PLAN --landing --changed-in $BASE..$TIP" "$MAIN" BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=1
hook_case "9 a branch push through bx plans without --landing" 0 \
    "$PLAN --changed-in $BASE..$TIP" "$TOPIC" BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=0
hook_case "10 in place, a main update hands the runner --landing, and its 3 is the hook's 3" 3 \
    "runner --landing --changed-in $BASE..$TIP" "$MAIN" PREPUSH_TEST_RUNNER_RC=3
hook_case "11 in place, a branch push's runner 2 is the hook's 2" 2 \
    "runner --changed-in $BASE..$TIP" "$TOPIC" PREPUSH_TEST_RUNNER_RC=2
hook_case "12 PREPUSH_FULL on a main update plans --all --landing, status as is" 2 \
    "$PLAN --all --landing" "$MAIN" BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=2 PREPUSH_FULL=1
# An item's tree is judged on its candidate, by the integrator's own run for refs/heads/main, so its
# push asks the stand-ins nothing, and a bx that would fail says nothing about it.
hook_case "13 a genuine landing queue item asks for no gate" 0 \
    "" "refs/heads/topic $TIP refs/bx-land/main/items/0000000001-abc $Z" \
    BX="$WORK/bin/bx" PREPUSH_TEST_BX_RC=1

# ── A landing counts no skip as a pass (scripts/gates/lib.sh) ────────────────────────────────────
#
# The skip sites of the gate library, sourced from this checkout and driven with a tool no machine
# has. In a landing (SCE_GATE_LANDING, which `scripts/gate --landing` exports) a skip ends 3, could
# not run, which bx reports as not measured; outside one it stays the skip it was. The mutant each
# landing case turns red on is the old `exit 0`.
LIB="$(cd "$HERE/../.." && pwd)/scripts/gates/lib.sh"
ABSENT="no-such-tool-in-this-fixture-qq"
cat >"$WORK/bin/suite-needs-a-tool" <<'STUB'
#!/usr/bin/env bash
# A suite whose case finds its tool missing: it skips, or fails naming the variable when told to.
if [[ -n "${SCE_REQUIRE_TOOLS:-}" ]]; then
    echo "case: tool not on PATH and SCE_REQUIRE_TOOLS is set"
    exit 101
fi
echo "warning: case skipped"
exit 0
STUB
cat >"$WORK/bin/suite-that-fails" <<'STUB'
#!/usr/bin/env bash
echo "case: assertion failed"
exit 101
STUB
cat >"$WORK/bin/suite-that-passes" <<'STUB'
#!/usr/bin/env bash
echo "case: passed"
exit 0
STUB
chmod +x "$WORK/bin/suite-needs-a-tool" "$WORK/bin/suite-that-fails" "$WORK/bin/suite-that-passes"

# lib_case <name> <want status> <shell text run after sourcing the library> [VAR=value...]
lib_case() {
    local name="$1" want_rc="$2" body="$3" rc=0
    shift 3
    env -u SCE_GATE_LANDING -u SCE_REQUIRE_TOOLS "$@" \
        bash -c 'source "$1"; eval "$2"' _ "$LIB" "$body" >"$WORK/lib.out" 2>&1 || rc=$?
    if [[ "$rc" == "$want_rc" ]]; then
        pass "$name"
    else
        fail "$name" "exited $rc, want $want_rc; its output:"
        sed 's/^/      /' "$WORK/lib.out"
    fi
}

lib_case "14 in a landing, a missing tool is could not run (3), never a skip's 0" 3 \
    "sce_gate_requires_tool $ABSENT some-package || exit 0" SCE_GATE_LANDING=1
lib_case "15 outside a landing, the same missing tool is the skip it was (0)" 0 \
    "sce_gate_requires_tool $ABSENT some-package || exit 0"
lib_case "16 outside a landing, SCE_REQUIRE_TOOLS still makes it a failure (1)" 1 \
    "sce_gate_requires_tool $ABSENT some-package || exit 0" SCE_REQUIRE_TOOLS=1
lib_case "17 in a landing, a skip of the gate is could not run (3)" 3 \
    "sce_gate_skip 'the thing is not here.'" SCE_GATE_LANDING=1
lib_case "18 outside a landing, a skip of the gate is a skip (0)" 0 \
    "sce_gate_skip 'the thing is not here.'"
lib_case "19 in a landing, a suite case that finds its tool missing is could not run (3)" 3 \
    "sce_gate_run_suite 'the suite' '$WORK/bin/suite-needs-a-tool'" SCE_GATE_LANDING=1
lib_case "20 in a landing, a suite that fails on its own is the tree's failure (1)" 1 \
    "sce_gate_run_suite 'the suite' '$WORK/bin/suite-that-fails'" SCE_GATE_LANDING=1
lib_case "21 outside a landing, a suite case that skips passes as it always has (0)" 0 \
    "sce_gate_run_suite 'the suite' '$WORK/bin/suite-needs-a-tool'"

# A gate that runs a suite also starts services it stops at its own exit (the W3C HTTP fixture
# server of `workspace-tests`). The library's suite runner must not wait for them: a landing's unit
# that does never ends, and the machine's idle limit ends it. The service here lives 60 s, and the
# suite is over in under one, so a runner that waits for it takes the minute and the case is red.
# The service is a child of the shell that runs the suite, which is what a bare `wait` waits for.
service_body='sleep 60 >/dev/null 2>&1 & sce_gate_on_exit "kill $! 2>/dev/null"; sce_gate_run_suite "the suite" "$SUITE"; (( SECONDS < 30 ))'
lib_case "23 in a landing, a suite that passes beside a service of the gate's returns" 0 \
    "$service_body" SCE_GATE_LANDING=1 SUITE="$WORK/bin/suite-that-passes"

# The runner is what turns `--landing` into SCE_GATE_LANDING for every gate it starts.
if grep -q '^    export SCE_GATE_LANDING=1$' "$HERE/../../scripts/gate"; then
    pass "22 the runner exports SCE_GATE_LANDING for --landing"
else
    fail "22 the runner exports SCE_GATE_LANDING for --landing" "scripts/gate no longer exports it"
fi

exit "$fails"
