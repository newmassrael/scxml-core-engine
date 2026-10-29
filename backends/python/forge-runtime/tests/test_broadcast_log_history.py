# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Python arm's stress runs of the `broadcast-log` kind, written as
# histories (RFC §synth-5-S; ADR 0062 section 5).
#
# These tests run the runtime log on real threads, record every operation each
# thread made, and write one history per run in the JSON form every backend
# writes (tests/forge/conformance/broadcast_history.schema.json). They judge
# nothing about linearizability: `sce-codegen check-broadcast-history` does, for
# this backend as for the others, so no backend carries a checker of its own.
# The Python gate (scripts/gates/forge-python.sh) sets
# SCE_BROADCAST_HISTORY_DIR, runs these, then runs the command over what they
# wrote. Without the variable there is nowhere to write, and the writing tests
# are skipped by name; the test of the recorder's shape does not need it.
#
# The recording follows the queue arm's: one counter that every thread reads
# immediately before and immediately after each call, so a read-modify-write
# chain on one lock orders the readings with the calls between them and
# "returned before invoked" in the record means it in the run. Every operation
# is recorded, the empty reads too, because those are results the checker must
# account for.
#
# The values published are a function of the sequence number, so a torn entry
# (words of two entries) is caught in the thread that read it, and a value the
# checker reads in a history is the one the publisher recorded for that
# sequence number. The histories are kept short: the checker's general search
# is exponential in the readers' operations in the worst case (ADR 0062 section
# 5), so the length is bounded by the number of publishes and by a pause after
# an empty read, not by the time the run takes.

from __future__ import annotations

import json
import os
import threading
import time
import unittest
from pathlib import Path
from typing import Any

from sce_forge_runtime.broadcast_log import BroadcastLog, Empty, Item, Lagged

DEADLINE_SECONDS = 120
WORDS = 2
# Runs of each shape: short runs, many of them, because the schedules that
# matter are a small fraction of those a machine gives.
RUNS_PER_SHAPE = 10
# (retention, readers, publishes): the ring under test holds retention + 1
# slots. A retention of 1 lags a reader after any two publishes it has not
# read; 8 lags one only when it is far behind.
SHAPES = (
    (1, 1, 200),
    (2, 2, 150),
    (3, 3, 100),
    (4, 3, 120),
    (8, 2, 100),
)
# How long a reader waits after finding the log empty, and the extra pause of a
# slow reader after each entry, in seconds. The slow reader is what makes a log
# of small retention lag it.
EMPTY_PAUSE = 0.0002
SLOW_PAUSE = 0.0003


class Clock:
    """The counter every thread reads before and after each call."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._ticks = 0

    def tick(self) -> int:
        with self._lock:
            self._ticks += 1
            return self._ticks


def _value_of(seq: int) -> int:
    """The value every word of the entry `seq` holds."""
    return seq + 1


def _record_run(run: int, retention: int, readers: int, publishes: int) -> dict[str, Any]:
    """One publisher publishing `publishes` entries and `readers` readers on real
    threads through a log of `retention + 1` slots, judged against a sequential
    log of `retention`. Reader `i` is opened at the oldest entry, the head, or a
    position, by `i` and the run number; the readers of odd index are slow."""
    log = BroadcastLog(WORDS, retention + 1)
    clock = Clock()
    deadline = time.monotonic() + DEADLINE_SECONDS
    done = threading.Event()
    failures: list[str] = []
    publisher_log: list[dict[str, Any]] = []
    reader_logs: list[list[dict[str, Any]]] = [[] for _ in range(readers)]

    def publish() -> None:
        handle = log.publisher()
        assert handle is not None
        with handle:
            for n in range(publishes):
                if time.monotonic() > deadline:
                    failures.append("the publisher did not finish before the deadline")
                    break
                value = _value_of(n)
                invoked = clock.tick()
                seq = handle.publish([value] * WORDS)
                returned = clock.tick()
                publisher_log.append(
                    {"call": "publish", "value": value, "seq": seq, "invoked": invoked, "returned": returned}
                )
                if n % 16 == 15:
                    invoked = clock.tick()
                    head = log.head()
                    returned = clock.tick()
                    publisher_log.append({"call": "head", "head": head, "invoked": invoked, "returned": returned})
                    time.sleep(0)
        done.set()

    def consume(i: int) -> None:
        mine = reader_logs[i]
        kind = (i + run) % 3
        invoked = clock.tick()
        if kind == 0:
            handle = log.reader_oldest()
            opened: dict[str, Any] = {"from": "oldest"}
        elif kind == 1:
            handle = log.reader_head()
            opened = {"from": "head"}
        else:
            at = run % (publishes // 2 + 1)
            handle = log.reader_at(at)
            opened = {"from": "at", "at": at}
        returned = clock.tick()
        mine.append({"call": "open", **opened, "cursor": handle.cursor, "invoked": invoked, "returned": returned})
        slow = i % 2 == 1
        out = [0] * WORDS
        reads = 0
        while True:
            finished = done.is_set()
            if time.monotonic() > deadline:
                failures.append(f"reader {i} did not finish before the deadline")
                return
            invoked = clock.tick()
            result = log.read(handle, out)
            returned = clock.tick()
            if isinstance(result, Item):
                value = out[0]
                if any(word != value for word in out) or value != _value_of(result.seq):
                    failures.append(f"reader {i} got the words {out} for the entry {result.seq}")
                mine.append(
                    {
                        "call": "read",
                        "outcome": "item",
                        "seq": result.seq,
                        "value": value,
                        "invoked": invoked,
                        "returned": returned,
                    }
                )
                if slow:
                    time.sleep(SLOW_PAUSE)
            elif isinstance(result, Lagged):
                mine.append(
                    {
                        "call": "read",
                        "outcome": "lagged",
                        "skipped": result.skipped,
                        "invoked": invoked,
                        "returned": returned,
                    }
                )
            else:
                assert isinstance(result, Empty)
                mine.append({"call": "read", "outcome": "empty", "invoked": invoked, "returned": returned})
                if finished:
                    return
                time.sleep(EMPTY_PAUSE)
            reads += 1
            if reads % 8 == 0:
                invoked = clock.tick()
                head = log.head()
                returned = clock.tick()
                mine.append({"call": "head", "head": head, "invoked": invoked, "returned": returned})

    threads = [threading.Thread(target=publish)]
    threads += [threading.Thread(target=consume, args=(i,)) for i in range(readers)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    if failures:
        raise AssertionError(failures[0])
    return {"version": 1, "retention": retention, "publisher": publisher_log, "readers": reader_logs}


def _history_dir() -> Path:
    named = os.environ.get("SCE_BROADCAST_HISTORY_DIR")
    if not named:
        raise unittest.SkipTest("SCE_BROADCAST_HISTORY_DIR names where the histories are written; it is unset")
    path = Path(named)
    path.mkdir(parents=True, exist_ok=True)
    return path


def _write(directory: Path, name: str, document: dict[str, Any]) -> None:
    (directory / f"{name}.json").write_text(json.dumps(document, separators=(",", ":")) + "\n")


# What each kind of operation carries beyond `call`, `invoked` and `returned`,
# exactly as the oneOf of the schema lists it.
FIELDS = {
    ("publish", None): {"value", "seq"},
    ("open", "oldest"): {"from", "cursor"},
    ("open", "head"): {"from", "cursor"},
    ("open", "at"): {"from", "at", "cursor"},
    ("read", "item"): {"outcome", "seq", "value"},
    ("read", "empty"): {"outcome"},
    ("read", "lagged"): {"outcome", "skipped"},
    ("head", None): {"head"},
}


def _kind_of(op: dict[str, Any]) -> tuple[str, str | None]:
    if op["call"] == "read":
        return ("read", op["outcome"])
    if op["call"] == "open":
        return ("open", op["from"])
    return (op["call"], None)


class TestRecorder(unittest.TestCase):
    def test_a_recorded_run_has_the_form_the_schema_describes(self) -> None:
        for retention, readers, publishes in ((1, 2, 40), (3, 3, 40)):
            with self.subTest(retention=retention):
                document = _record_run(0, retention, readers, publishes)
                self.assertEqual({"version", "retention", "publisher", "readers"}, set(document))
                self.assertEqual(1, document["version"])
                self.assertEqual(retention, document["retention"])
                self.assertEqual(readers, len(document["readers"]))
                participants = [document["publisher"], *document["readers"]]
                readings: list[int] = []
                for ops in participants:
                    previous = 0
                    for op in ops:
                        kind = _kind_of(op)
                        self.assertIn(kind, FIELDS)
                        self.assertEqual({"call", "invoked", "returned"} | FIELDS[kind], set(op), op)
                        self.assertLess(op["invoked"], op["returned"])
                        self.assertLess(previous, op["invoked"])
                        previous = op["returned"]
                        readings += [op["invoked"], op["returned"]]
                self.assertEqual(len(readings), len(set(readings)), "the clock gives distinct readings")
                publishes_made = [op for op in document["publisher"] if op["call"] == "publish"]
                self.assertEqual(publishes, len(publishes_made))
                self.assertEqual(list(range(publishes)), [op["seq"] for op in publishes_made])
                self.assertEqual([_value_of(n) for n in range(publishes)], [op["value"] for op in publishes_made])
                for ops in document["readers"]:
                    self.assertEqual("open", ops[0]["call"], "a reader is opened before it reads")
                    self.assertEqual(["open"], [op["call"] for op in ops if op["call"] == "open"])
                    self.assertEqual("empty", ops[-1]["outcome"], "a reader stops at the head of a finished publisher")

    def test_the_document_is_valid_json_with_the_integers_the_schema_asks_for(self) -> None:
        document = _record_run(1, 2, 2, 30)
        text = json.dumps(document, separators=(",", ":"))
        self.assertEqual(document, json.loads(text))


class TestBroadcastLogHistories(unittest.TestCase):
    def test_runs_are_written_as_histories(self) -> None:
        directory = _history_dir()
        for retention, readers, publishes in SHAPES:
            for run in range(RUNS_PER_SHAPE):
                document = _record_run(run, retention, readers, publishes)
                _write(directory, f"python_lock_r{retention}_c{readers}_{run}", document)


if __name__ == "__main__":
    unittest.main()
