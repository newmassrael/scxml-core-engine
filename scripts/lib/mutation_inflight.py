#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""What a mutation round is doing RIGHT NOW, written where a later one can read it.

`scripts/lib/mutation_ledger.sh` records what a round CONCLUDED. This records
what it is in the middle of, and it exists for the half of a round's evidence a
verdict cannot carry: a round that is killed reaches no verdict, and — until
this file — reached no restore either.

`scripts/mutate` hangs every restore off one `trap ... EXIT`. A trap does not
run on SIGKILL, so a round killed outright leaves its last mutation in the
working tree. Both consequences read as success:

  - the next round takes that mutated file as its baseline, and whatever it
    then reports is a measurement of a tree nobody chose;
  - a successor session reads the edit as leftovers and reverts it by hand.
    Twice, measured. When the round was in fact still alive, that revert
    produced a false CAUGHT, which is a verdict that reads as a test working.

What made this unrecoverable was never lost data. `scripts/mutate` copies every
declared file before the first case, and that snapshot — the original bytes,
exactly — survives the kill intact under its `mktemp -d` directory. Only its
NAME dies, because the name lived nowhere but in the killed process's memory.

So the round writes the name down, before it touches a file, at a path with no
session in it: the same argument and the same root as the verdict ledger's.
The record is removed once the restore is done AND verified, so the records
still present are exactly the rounds that did not finish.

Four subcommands. `open` and `case` are called by a round, `close` from its
EXIT trap, and `recover` by the NEXT invocation of the harness — which is the
one that pays this file's debts, because it is the one that is still alive.

Output is tagged so the caller can speak in its own voice: each report line is
`<R|G|D><TAB><text>`, for the caller's red, green and dim. Exit status is 0
when nothing is outstanding and 1 when this file refuses to guess.
"""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
import time

TAG_RED = "R"
TAG_GREEN = "G"
TAG_DIM = "D"


def emit(tag, text):
    sys.stdout.write("%s\t%s\n" % (tag, text))


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 16), b""):
            digest.update(chunk)
    return digest.hexdigest()


def boot_id():
    """Which boot this is.

    A pid plus its start time is unique for as long as a boot lasts and not one
    moment longer, so the boot is part of the identity or the identity is a
    coincidence waiting to happen.
    """
    try:
        with open("/proc/sys/kernel/random/boot_id") as handle:
            return handle.read().strip()
    except OSError:
        return "unknown"


def proc_stat(pid):
    """The fields of /proc/<pid>/stat from the state onwards, or None.

    Sliced after the LAST `)` rather than counted from the front: the comm
    field is parenthesised and may itself contain spaces and parentheses,
    which is exactly what breaks a plain field count. What remains starts at
    field 3, so field N of the manual page is at index N - 3 here.
    """
    try:
        with open("/proc/%d/stat" % pid) as handle:
            raw = handle.read()
    except OSError:
        return None
    try:
        return raw.rsplit(") ", 1)[1].split()
    except IndexError:
        return None


def start_time(pid):
    """When a process started, in ticks since boot, or None if unreadable."""
    fields = proc_stat(pid)
    if fields is None or len(fields) < 20:
        return None
    return fields[19]


def process_state(pid):
    """The process's state letter, or None if unreadable."""
    fields = proc_stat(pid)
    if not fields:
        return None
    return fields[0]


def process_is_alive(record):
    """Is the process this record names still the process that wrote it?

    Every uncertain answer here is "alive", deliberately. Refusing to touch a
    round that has in fact died costs one message on the next invocation;
    reverting one that is in fact live is the failure this whole file exists to
    stop, and it produces a verdict rather than an error.
    """
    pid = record.get("pid")
    if not isinstance(pid, int) or pid <= 0:
        return True
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except OSError:
        return True
    # A killed round whose parent has not reaped it is a zombie, and a zombie
    # answers `kill -0` and keeps its /proc entry. Reading that as "still
    # running" is the one way this check fails in the direction that leaves the
    # mutation in the tree forever, so the state letter is asked for by name.
    if process_state(pid) in ("Z", "X"):
        return False
    recorded = record.get("starttime")
    current = start_time(pid)
    if not recorded or not current:
        return True
    return record.get("boot_id") == boot_id() and recorded == current


def describe(record):
    """The round, in the words a reader needs to decide what it was."""
    what = record.get("casefile") or "(an unnamed casefile)"
    if record.get("mode") and record["mode"] != "run":
        what += " (--%s)" % record["mode"]
    if record.get("case"):
        what += ", applying %r" % record["case"]
    return what


def outstanding(record):
    """The declared paths whose bytes are not the ones the round started from.

    Compared against the hashes taken WITH the snapshot, not against git: a
    round is entitled to start from a dirty tree, and "back where it was" for
    such a round is that dirty content and not HEAD's.
    """
    left = []
    for target in record.get("targets") or []:
        path, want = target.get("path"), target.get("sha256")
        try:
            have = sha256(path)
        except OSError:
            have = None
        if have != want:
            left.append(path)
    return left


def discard_scratch(record):
    """Remove the round's `mktemp -d` directory — the `rm -rf` its trap missed.

    Only when the snapshot this file just read lives inside it, so a record
    naming some other directory removes nothing.
    """
    work = record.get("work") or ""
    snapshot = record.get("snapshot") or ""
    if not work or not snapshot:
        return
    if not snapshot.startswith(work.rstrip(os.sep) + os.sep):
        return
    shutil.rmtree(work, ignore_errors=True)


def write_record(path, record):
    """Replace the record in one step.

    A kill landing in the middle of a rewrite must leave the PREVIOUS record,
    not half of one: the record's whole worth is that it can be read by
    something that did not write it.
    """
    scratch = path + ".new"
    with open(scratch, "w") as handle:
        json.dump(record, handle, ensure_ascii=False)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(scratch, path)


def load(path):
    with open(path) as handle:
        return json.load(handle)


def retire(path):
    """Remove a record that has been accounted for.

    Tolerant of the file already being gone: two harnesses can start at once,
    and both are entitled to conclude that the same abandoned round has been
    put right.
    """
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass


def cmd_open(args):
    """Write the record, and print the path it landed at.

    The NAME is chosen here rather than by a `mktemp` in the caller, because a
    caller that creates the file first leaves a zero-byte `.json` sitting in the
    directory until the content arrives — and a concurrent invocation that reads
    it in that instant sees not a record but a parse error, and refuses. Measured
    2026-08-26 running this repository's own harness suites five at a time.
    Staged under a name `recover` does not consider, and moved into place in one
    step, a record is either absent or complete.
    """
    targets = []
    for line in sys.stdin.read().splitlines():
        if not line:
            continue
        targets.append({"path": line, "sha256": sha256(line)})
    os.makedirs(args.dir, exist_ok=True)
    handle, staging = tempfile.mkstemp(prefix="round-", suffix=".json.new", dir=args.dir)
    os.close(handle)
    record = staging[: -len(".new")]
    write_record(
        record,
        {
            "schema": 1,
            "pid": args.pid,
            "starttime": start_time(args.pid),
            "boot_id": boot_id(),
            "host": os.uname().nodename,
            "repo": args.repo,
            "casefile": args.casefile,
            "mode": args.mode,
            "tree": args.tree,
            "work": args.work,
            "snapshot": args.snapshot,
            "started": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "targets": targets,
        },
    )
    # No cleanup: `write_record` stages at `<record>.new`, which IS the file
    # `mkstemp` just made, and moves it into place.
    print(record)
    return 0


def cmd_case(args):
    """Name the case now being applied, so a refusal can say which one it was.

    Which case a killed round died in is the difference between "something in
    this casefile is in your tree" and a sentence a reader can act on, and it is
    also the one case in the casefile that has no verdict.
    """
    try:
        record = load(args.record)
    except (OSError, ValueError):
        return 0
    record["case"] = args.label
    write_record(args.record, record)
    return 0


def cmd_close(args):
    """Remove the record — but only once the tree it named is back.

    The restore runs first, in the same EXIT trap, and `mutation_restore_quiet`
    swallows its errors by design. Removing the record unconditionally after it
    would hand the failure mode straight back: a restore that did not happen and
    no record left saying so. Keeping the record is also what tells the caller
    not to delete the snapshot the repair would need.
    """
    try:
        record = load(args.record)
    except OSError:
        return 0
    except ValueError as exc:
        emit(TAG_RED, "the in-flight record %s is unreadable (%s), so this round"
             % (args.record, exc))
        emit(TAG_RED, "cannot say whether it put the tree back. It is kept.")
        return 1
    left = outstanding(record)
    if not left:
        retire(args.record)
        return 0
    emit(TAG_RED, "the round ended with its mutation still in the tree:")
    for path in left:
        emit(TAG_RED, "  %s" % path)
    emit(TAG_RED, "the record that says how to put it back is kept at")
    emit(TAG_RED, "  %s" % args.record)
    emit(TAG_RED, "and so is the snapshot it names. The next `scripts/mutate` in this")
    emit(TAG_RED, "tree restores from it.")
    return 1


def cmd_recover(args):
    """Find the rounds that did not finish, and finish their restore.

    Runs before this invocation touches anything, because a leftover mutation
    that reaches the baseline is not a stale file — it is a measurement of the
    wrong tree, reported with the same confidence as any other.
    """
    if not os.path.isdir(args.dir):
        return 0
    refused = False
    for name in sorted(os.listdir(args.dir)):
        if not name.endswith(".json"):
            continue
        path = os.path.join(args.dir, name)
        try:
            record = load(path)
        except FileNotFoundError:
            # Another invocation repaired this one between the listing and the
            # read. A record that is GONE is the outcome this asks for, not a
            # record that cannot be read, and refusing here would turn two
            # harnesses running at once into a failure of both.
            continue
        except (OSError, ValueError) as exc:
            emit(TAG_RED, "an in-flight round record cannot be read: %s" % path)
            emit(TAG_RED, "  %s" % exc)
            emit(TAG_RED, "It is left where it is: an unreadable record is still the"
                 " only trace of")
            emit(TAG_RED, "whatever round wrote it, and deleting it here would leave"
                 " none.")
            refused = True
            continue
        # Another checkout's round. Its own harness owns that restore, and this
        # one has neither its snapshot's tree nor any business in it.
        if record.get("repo") != args.repo:
            continue
        # A round that is still running is not this invocation's business, and
        # saying so here would be wrong twice over: it is not a repair, and it
        # would make one harness's output depend on what else happens to be
        # running — which is how five concurrent `--check` runs came to fail on
        # each other's notes. Whoever wants to know asks
        # `scripts/mutation-ledger in-flight`, which exists for that question.
        if process_is_alive(record):
            continue
        if not handle_abandoned(path, record):
            refused = True
    return 1 if refused else 0


def cmd_list(args):
    """Every record, and whether its round is still going.

    The question `scripts/mutate` deliberately does not answer. A round in
    flight is not a repair, so reporting it from a harness that is trying to
    start one makes that harness's output depend on what else is running — but
    the question itself is the one a session opens with, because a dirty tree
    that belongs to a LIVE round has been reverted by a successor twice.
    """
    if not os.path.isdir(args.dir):
        return 0
    for name in sorted(os.listdir(args.dir)):
        if not name.endswith(".json"):
            continue
        path = os.path.join(args.dir, name)
        try:
            record = load(path)
        except (OSError, ValueError):
            print("unreadable\t%s" % path)
            continue
        print(
            "%s\tpid=%s\t%s\t%s\t%s"
            % (
                "running" if process_is_alive(record) else "abandoned",
                record.get("pid"),
                record.get("started"),
                describe(record),
                record.get("repo"),
            )
        )
    return 0


def handle_abandoned(path, record):
    """One record whose round is gone. True when nothing is left outstanding."""
    left = outstanding(record)
    if not left:
        retire(path)
        discard_scratch(record)
        emit(TAG_DIM, "cleared the record of a round that died without leaving a"
             " mutation behind: %s" % describe(record))
        return True

    snapshot = record.get("snapshot") or ""
    restorable, unreachable = [], []
    for target in record.get("targets") or []:
        if target.get("path") not in left:
            continue
        source = os.path.join(snapshot, target["path"])
        try:
            if sha256(source) == target.get("sha256"):
                restorable.append((target["path"], source, target["sha256"]))
                continue
        except OSError:
            pass
        unreachable.append(target["path"])

    if unreachable:
        emit(TAG_RED, "a mutation round was killed in this tree and its mutation is"
             " still here:")
        for still_there in left:
            emit(TAG_RED, "  %s" % still_there)
        emit(TAG_RED, "the round was %s" % describe(record))
        emit(TAG_RED, "and the bytes it started from are not where it said they would"
             " be:")
        emit(TAG_RED, "  %s" % (snapshot or "(the record names no snapshot)"))
        emit(TAG_RED, "This harness will not guess at them. `git checkout` would put"
             " back HEAD,")
        emit(TAG_RED, "which is not what the round started from whenever the tree was"
             " dirty, and")
        emit(TAG_RED, "would discard whatever else those files carried. Put them back"
             " by hand and")
        emit(TAG_RED, "run again: this refusal clears itself once their bytes match"
             " what the round")
        emit(TAG_RED, "recorded. The record is kept at %s" % path)
        return False

    for target_path, source, want in restorable:
        with open(source, "rb") as handle:
            original = handle.read()
        # Into the existing file, so its mode survives — the reason
        # `mutation_restore` copies rather than moves.
        with open(target_path, "wb") as handle:
            handle.write(original)
        if sha256(target_path) != want:
            emit(TAG_RED, "restoring %s did not reproduce the bytes the round"
                 " recorded." % target_path)
            emit(TAG_RED, "The record is kept at %s" % path)
            return False

    emit(TAG_GREEN, "restored %d file(s) left mutated by a round that was killed,"
         " from that round's own snapshot:" % len(restorable))
    for target_path, _, _ in restorable:
        emit(TAG_GREEN, "  %s" % target_path)
    emit(TAG_DIM, "  the round was %s" % describe(record))
    emit(TAG_DIM, "  started %s, pid %s, and reached no verdict"
         % (record.get("started"), record.get("pid")))
    retire(path)
    discard_scratch(record)
    return True


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)

    opener = sub.add_parser("open", help="write the record; targets on stdin")
    opener.add_argument("--dir", required=True)
    opener.add_argument("--repo", required=True)
    opener.add_argument("--casefile", required=True)
    opener.add_argument("--mode", required=True)
    opener.add_argument("--tree", required=True)
    opener.add_argument("--work", required=True)
    opener.add_argument("--snapshot", required=True)
    opener.add_argument("--pid", required=True, type=int)
    opener.set_defaults(func=cmd_open)

    case = sub.add_parser("case", help="name the case now being applied")
    case.add_argument("--record", required=True)
    case.add_argument("--label", required=True)
    case.set_defaults(func=cmd_case)

    close = sub.add_parser("close", help="remove the record once the tree is back")
    close.add_argument("--record", required=True)
    close.set_defaults(func=cmd_close)

    recover = sub.add_parser("recover", help="finish the restore of rounds that died")
    recover.add_argument("--dir", required=True)
    recover.add_argument("--repo", required=True)
    recover.set_defaults(func=cmd_recover)

    listing = sub.add_parser("list", help="one line per record, running or not")
    listing.add_argument("--dir", required=True)
    listing.set_defaults(func=cmd_list)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
