# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Where a mutation round's verdict goes, so the NEXT session can read it.
#
# A round is the most expensive thing this repository does — a rebuild and a
# suite run per case — and until this file existed its result was printed to a
# terminal and nowhere else. Whoever ran it wrote the outcome down by hand,
# into a scratch directory keyed by the session id. That is not a filing
# habit; it is the mechanism behind two measured failures:
#
#   1. **The corpus had to be re-derived every session.** "Which casefiles
#      have been judged" lived only in the previous session's scratch
#      directory, so the answer was reconstructed from memory notes that are
#      stale by construction. Measured 2026-08-26: the debt registry's own
#      residue count had missed a batch of five casefiles entirely, and the
#      surviving evidence for a further twelve was a name in a list whose
#      round log no longer exists.
#
#   2. **A successor session read a live round's mutation as leftovers and
#      reverted it.** Twice. The round's log, pid and exit status were in the
#      outgoing session's scratch directory, so the incoming session could not
#      see that a round was in flight; it saw only an edited working tree. The
#      verdict that came back was false in both directions — a mutation
#      reverted before the build reads as SURVIVED, one reverted during the
#      build reads as CAUGHT, and the second is worse because it reads as
#      success.
#
# Both are the same root cause: the evidence was written where only its author
# could find it. So the harness writes it itself, to a path with no session in
# it, and a round that never finishes leaves no record at all — which is the
# honest answer, because a round that did not finish read no verdict.
#
# ── Where ─────────────────────────────────────────────────────────
#
# Outside the git tree, deliberately. A verdict is a measurement of a tree at
# a commit, not a fact about the source, and committing it would make every
# round a working-tree change the next round has to reason about: this harness
# restores the files it mutates and compares them against a baseline, and a
# ledger inside that tree is one more thing that can differ.
#
# `$HOME/.local/share`, spelled out, and NOT `$XDG_DATA_HOME`. That was the
# first shape of this rule and it was wrong in the precise way the whole file
# is about: measured 2026-08-26, the loop harness that drives these rounds
# exports `XDG_DATA_HOME=$HOME/.local/share/sprag-loop/data` into its panes,
# so the first round written through this library landed under the harness's
# own per-run directory instead of beside the corpus — a path that moves with
# whoever is running, which is a session id by another name.
#
# `$HOME/.local/share/sce-mutation-corpus/` is where this corpus's rescued
# evidence already lives, and it is the same directory whether a round is run
# by a person at a shell, by the loop, or by a gate.
#
# ── What one line says ────────────────────────────────────────────
#
# One JSON object per round, appended to `<stem>.jsonl`. Append-only: a
# casefile judged at three commits has three lines, and the newest is not the
# only one worth reading — a case CAUGHT in June and SURVIVED in August is
# exactly the signal a single mutable record would erase.
#
# Every line carries what it takes to decide whether it still describes the
# tree in front of you: the commit the round was measured against, whether
# that tree was dirty, the casefile's own blob hash (so an edited casefile
# makes its old verdict visibly stale), and where the reading came from —
# `live` for a round this harness ran, `round-log` for one recovered from a
# saved console log, `asserted` for a name someone recorded without a log.
# The three are not equivalent, and a ledger that could not tell them apart
# would launder the third into the first.

# The corpus's own directory, which both records below hang off. Spelled once
# because the argument above is about the ROOT — a path with no session in it —
# and a second copy is how one of the two records quietly moves.
mutation_ledger_root() {
    printf '%s\n' "$HOME/.local/share/sce-mutation-corpus"
}

# The ledger's location, and the ONLY place it is spelled. `scripts/mutate`
# writes through this file and `scripts/mutation-ledger` reads through it, so
# neither can drift from the other by editing a path.
mutation_ledger_dir() {
    if [[ -n "${SCE_MUTATION_LEDGER_DIR:-}" ]]; then
        printf '%s\n' "$SCE_MUTATION_LEDGER_DIR"
        return
    fi
    printf '%s\n' "$(mutation_ledger_root)/verdicts"
}

# Field separators for the scratch file the case loop appends to. A label is
# author-written prose, so the separators are characters prose does not
# contain: US between a row's fields, RS between the names inside one field.
MUTATION_LEDGER_FS=$'\037'
MUTATION_LEDGER_RS=$'\036'

# Begin a round. Records the tree the round is measured AGAINST, read here
# rather than at the end because by then the harness has mutated and restored
# the tree once per case — and a restore that did not reproduce the baseline
# is precisely the condition under which an end-of-round reading would lie.
mutation_ledger_begin() {
    local casefile="$1" scratch="$2"
    MUTATION_LEDGER_CASEFILE="$casefile"
    MUTATION_LEDGER_ROWS="$scratch"
    : >"$MUTATION_LEDGER_ROWS"
    MUTATION_LEDGER_STARTED="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    MUTATION_LEDGER_TREE="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
    if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
        MUTATION_LEDGER_DIRTY=true
    else
        MUTATION_LEDGER_DIRTY=false
    fi
    MUTATION_LEDGER_BLOB="$(git hash-object "$casefile" 2>/dev/null || echo unknown)"
}

# One case's verdict, as it is printed. `detail` is the parenthetical the
# console shows (`1/2 red`) or, for an INCONCLUSIVE, the one-line reason the
# harness gives underneath — the part that says which of the ways a case can
# fail to be evidence this one was, and the part a bare count throws away.
#
# `red_file`, when the runner named what turned red, carries those names. A
# CAUGHT verdict's worth is not the count but whether the tests that turned
# red are the ones that own the clause: a mutation caught only by an unrelated
# suite is a case aimed at nothing in particular, and the names are the only
# place that shows.
mutation_ledger_case() {
    local verdict="$1" label="$2" detail="${3:-}" red_file="${4:-}" red=""
    [[ -n "${MUTATION_LEDGER_ROWS:-}" ]] || return 0
    if [[ -n "$red_file" && -s "$red_file" ]]; then
        red="$(tr '\n' "$MUTATION_LEDGER_RS" <"$red_file")"
    fi
    printf '%s%s%s%s%s%s%s\n' \
        "$verdict" "$MUTATION_LEDGER_FS" \
        "$label" "$MUTATION_LEDGER_FS" \
        "$detail" "$MUTATION_LEDGER_FS" \
        "$red" >>"$MUTATION_LEDGER_ROWS"
}

# Close the round: turn the rows into one JSON line and append it.
#
# Called only once the exit status is known, because the status is half of
# what a later reader needs — `caught=6 survived=0` and `rc=0` are not the
# same claim — and a round that ended some other way (a restore that did not
# reproduce the baseline, a killed runner) must not leave a record that reads
# like a completed one.
mutation_ledger_commit() {
    local runner="$1" rc="$2" provenance="${3:-live}"
    [[ -n "${MUTATION_LEDGER_ROWS:-}" ]] || return 0
    local dir
    dir="$(mutation_ledger_dir)"
    mkdir -p "$dir"
    MUTATION_LEDGER_FS="$MUTATION_LEDGER_FS" \
    MUTATION_LEDGER_RS="$MUTATION_LEDGER_RS" \
    python3 - "$MUTATION_LEDGER_ROWS" "$dir" "$MUTATION_LEDGER_CASEFILE" \
        "$MUTATION_LEDGER_TREE" "$MUTATION_LEDGER_DIRTY" "$MUTATION_LEDGER_BLOB" \
        "$runner" "$rc" "$provenance" "$MUTATION_LEDGER_STARTED" \
        "${MUTATION_LEDGER_NOTE:-}" <<'PY'
import json
import os
import sys
import time

(rows_path, dir_, casefile, tree, dirty, blob, runner, rc, provenance,
 started, note) = sys.argv[1:12]
fs = os.environ["MUTATION_LEDGER_FS"]
rs = os.environ["MUTATION_LEDGER_RS"]

cases = []
with open(rows_path) as fh:
    for line in fh:
        line = line.rstrip("\n")
        if not line:
            continue
        verdict, label, detail, red = line.split(fs)
        case = {"verdict": verdict, "label": label}
        if detail:
            case["detail"] = detail
        names = [n for n in red.split(rs) if n]
        if names:
            case["red"] = names
        cases.append(case)

tally = {"CAUGHT": 0, "SURVIVED": 0, "INCONCLUSIVE": 0}
for case in cases:
    if case["verdict"] in tally:
        tally[case["verdict"]] += 1

stem = os.path.basename(casefile)
if stem.endswith(".cases"):
    stem = stem[: -len(".cases")]

record = {
    "schema": 1,
    "stem": stem,
    "casefile": casefile,
    "tree": tree,
    "casefile_blob": blob,
    "runner": runner,
    "rc": int(rc),
    "provenance": provenance,
    "started": started,
    "ended": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "host": os.uname().nodename,
    "caught": tally["CAUGHT"],
    "survived": tally["SURVIVED"],
    "inconclusive": tally["INCONCLUSIVE"],
    "cases": cases,
}
# Whether the tree was dirty is an observation about a tree this record can
# name. A recovered round cannot — its log does not say which commit it ran
# against — and answering "clean" there would be a fact invented at import
# time. An absent field says "not observed"; `false` would say "observed
# clean", and the two must not be spelled the same.
if tree != "unknown":
    record["tree_dirty"] = dirty == "true"
# Only when there is one. A `note` is what a record carries INSTEAD of a
# reading — where the claim came from, when no round log survives — and an
# empty one on every live round would invite reading its absence as meaning
# something.
if note:
    record["note"] = note

# Appended, and flushed before the process leaves. A round is minutes of
# machine time; losing its one line to a buffer is not a trade worth making.
out = os.path.join(dir_, stem + ".jsonl")
with open(out, "a") as fh:
    fh.write(json.dumps(record, ensure_ascii=False) + "\n")
    fh.flush()
    os.fsync(fh.fileno())
print(out)
PY
}

# ── The other record: what a round is doing WHILE it does it ──────
#
# Everything above is written when a round ENDS. A round that is killed reaches
# none of it — and, until the record below existed, reached no restore either:
# `scripts/mutate` hangs every restore off one `trap ... EXIT`, and a trap does
# not run on SIGKILL. The mutation stays in the working tree, where the next
# round reads it as the baseline and a successor session reads it as leftovers.
#
# It is the same failure this file's header is about, one step earlier: the
# evidence that would have fixed it — the snapshot holding the original bytes —
# survives the kill intact, under a `mktemp -d` name that lived nowhere but in
# the killed process's memory. So the name is written down, before a file is
# touched, under the same root and for the same reason as a verdict.
#
# The record's mechanics are in `scripts/lib/mutation_inflight.py`, whose header
# carries the measurement; the four functions below are how a round reaches it.

# Plain-text stand-ins for the three helpers `scripts/mutate` defines further
# down its own file. It redefines them with colour once it reaches them, so
# these are what a reader sourcing this library on its own gets — rather than an
# unbound function on the one path that matters most, the report about a round
# that was killed.
declare -F red >/dev/null || red() { printf '%s\n' "$*"; }
declare -F green >/dev/null || green() { printf '%s\n' "$*"; }
declare -F dim >/dev/null || dim() { printf '%s\n' "$*"; }

# Where the in-flight records live: beside the verdicts, under the same root,
# for the same reason.
mutation_inflight_dir() {
    if [[ -n "${SCE_MUTATION_INFLIGHT_DIR:-}" ]]; then
        printf '%s\n' "$SCE_MUTATION_INFLIGHT_DIR"
        return
    fi
    printf '%s\n' "$(mutation_ledger_root)/in-flight"
}

# The program that owns a record's whole life, named once so the callers below
# cannot disagree about where it is.
mutation_inflight_tool() {
    printf '%s\n' "$(dirname "${BASH_SOURCE[0]}")/mutation_inflight.py"
}

# Speak the tool's report in this harness's voice.
#
# The tool tags its lines rather than colouring them, because it is read by a
# test and by a gate as well as by a person, and an escape sequence in front of
# a line is how `^applies` came to match nothing across a corpus that had just
# reported 124 cases.
mutation_inflight_speak() {
    local tag text
    while IFS=$'\t' read -r tag text; do
        case "$tag" in
        R) red "$text" ;;
        G) green "$text" ;;
        *) dim "$text" ;;
        esac
    done
}

# Open the round's record. Called from `mutation_snapshot`, which is the one
# moment both halves of it are true: the snapshot exists, and no case body has
# run yet.
#
# `$$` rather than a pid the tool could read for itself: `$$` is the shell
# running `scripts/mutate` even from inside a subshell, and that process is what
# a later invocation asks about when it wants to know whether the round that
# wrote this record is still going.
mutation_inflight_open() {
    local casefile="$1" mode="$2" work="$3" snapshot="$4" targets="$5"
    # The tool chooses the NAME as well as the content, and prints it back. A
    # name minted here would have to be created here, and a zero-byte `.json`
    # sitting in that directory until the content arrives is a record a
    # concurrent invocation reads as a parse error rather than as a round.
    MUTATION_INFLIGHT_RECORD="$(printf '%s\n' "$targets" |
        python3 "$(mutation_inflight_tool)" open \
        --dir "$(mutation_inflight_dir)" \
        --repo "$(git rev-parse --show-toplevel)" \
        --casefile "$casefile" \
        --mode "$mode" \
        --tree "$(git rev-parse HEAD 2>/dev/null || echo unknown)" \
        --work "$work" \
        --snapshot "$snapshot" \
        --pid "$$")"
}

# Every record this machine holds, and whether its round is still going.
#
# Read by `scripts/mutation-ledger in-flight` and by nothing else. A round in
# flight is deliberately NOT reported by `scripts/mutate`: it is not something
# that harness can act on, and reporting it there made one round's output depend
# on what else happened to be running.
mutation_inflight_list() {
    python3 "$(mutation_inflight_tool)" list --dir "$(mutation_inflight_dir)"
}

# Which case is being applied right now. Rewritten per case rather than written
# once, because "something in this casefile is in your tree" and "this case is
# in your tree" are a paragraph of searching apart — and the named case is also
# the one case in the round that has no verdict.
mutation_inflight_case() {
    [[ -n "${MUTATION_INFLIGHT_RECORD:-}" ]] || return 0
    python3 "$(mutation_inflight_tool)" case \
        --record "$MUTATION_INFLIGHT_RECORD" --label "$1"
}

# Close the record, once the tree it named is back. Non-zero when it is not —
# which is also what tells the EXIT trap to keep the snapshot a repair needs.
mutation_inflight_close() {
    [[ -n "${MUTATION_INFLIGHT_RECORD:-}" ]] || return 0
    local report rc=0
    report="$(python3 "$(mutation_inflight_tool)" close \
        --record "$MUTATION_INFLIGHT_RECORD")" || rc=$?
    if [[ -n "$report" ]]; then
        printf '%s\n' "$report" | mutation_inflight_speak
    fi
    if [[ "$rc" -eq 0 ]]; then
        MUTATION_INFLIGHT_RECORD=""
    fi
    return "$rc"
}

# Finish the restore of any round that did not. Non-zero when a record is left
# that this harness will not act on by guessing.
mutation_inflight_recover() {
    local report rc=0
    report="$(python3 "$(mutation_inflight_tool)" recover \
        --dir "$(mutation_inflight_dir)" \
        --repo "$(git rev-parse --show-toplevel)")" || rc=$?
    if [[ -n "$report" ]]; then
        printf '%s\n' "$report" | mutation_inflight_speak
    fi
    return "$rc"
}
