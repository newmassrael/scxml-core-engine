# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Python arm's stress runs of the `queue` kind, written as histories (SCE
# Protocol-Synthesis RFC §synth-5-P, verification layer 2).
#
# These tests run the runtime queue on real threads, record every attempt each
# thread made, and write one history per run in the JSON form every backend
# writes (tests/forge/conformance/queue_history.schema.json). They judge nothing
# about linearizability: `sce-codegen check-queue-history` does, for this backend
# as for the others, so no backend carries a checker of its own. The Python gate
# (scripts/gates/forge-python.sh) sets SCE_QUEUE_HISTORY_DIR, runs these, then
# runs the command over what they wrote. Without the variable there is nowhere
# to write, and the tests are skipped by name.
#
# The recording follows the Rust arm's: one counter that every thread reads
# immediately before and immediately after each call, so a read-modify-write
# chain on one lock orders the readings with the calls between them and
# "returned before invoked" in the record means it in the run. Every attempt is
# recorded, the refused pushes and the empty pops too, because those are results
# the checker must account for.
#
# The queue is one ring under one lock, so a push is refused only at capacity:
# the histories are judged under the `at-capacity` refusal, the strict one.

from __future__ import annotations

import json
import os
import threading
import time
import unittest
from pathlib import Path
from typing import Any

from sce_forge_runtime.queue import PushStatus, Queue

DEADLINE_SECONDS = 120
# Runs of each single-producer, single-consumer capacity. Their histories are
# megabytes and the ring has no schedule a second run reaches that the first
# does not.
RUNS_PER_ONE_ONE_CAPACITY = 3
VALUES_PER_ONE_ONE_RUN = 2000
# Runs of each many-participant shape: short runs, many of them, because the
# schedules that matter are a small fraction of those a machine gives.
RUNS_PER_SHAPE = 25
# (capacity, producers, consumers, values per producer), the Rust arm's shapes.
SHAPES = (
    (1, 2, 2, 150),
    (3, 2, 2, 150),
    (8, 2, 2, 150),
    (2, 3, 1, 120),
    (4, 1, 3, 120),
    (5, 2, 2, 150),
)


class Clock:
    """The counter every thread reads before and after each call."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._ticks = 0

    def tick(self) -> int:
        with self._lock:
            self._ticks += 1
            return self._ticks


def _record_run(capacity: int, producers: int, consumers: int, per_producer: int) -> dict[str, Any]:
    """`producers` producers and `consumers` consumers on real threads, each
    producer pushing `per_producer` distinct values."""
    queue: Queue[int] = Queue(capacity, producers, consumers)
    clock = Clock()
    deadline = time.monotonic() + DEADLINE_SECONDS
    total = producers * per_producer
    delivered = [0]
    delivered_lock = threading.Lock()
    failures: list[str] = []
    logs: list[list[dict[str, Any]]] = [[] for _ in range(producers + consumers)]

    def produce(p: int) -> None:
        handle = queue.producer()
        assert handle is not None
        with handle:
            for i in range(per_producer):
                value = p * per_producer + i + 1
                while True:
                    if time.monotonic() > deadline:
                        failures.append("the elements did not all arrive before the deadline")
                        return
                    invoked = clock.tick()
                    status = handle.try_push(value)
                    returned = clock.tick()
                    outcome = "pushed" if status is PushStatus.OK else "full"
                    logs[p].append(
                        {"call": "push", "value": value, "outcome": outcome, "invoked": invoked, "returned": returned}
                    )
                    if status is PushStatus.OK:
                        break
                    time.sleep(0)

    def consume(c: int) -> None:
        handle = queue.consumer()
        assert handle is not None
        mine = logs[producers + c]
        with handle:
            while True:
                with delivered_lock:
                    if delivered[0] >= total:
                        return
                if time.monotonic() > deadline:
                    failures.append("the elements did not all arrive before the deadline")
                    return
                invoked = clock.tick()
                value = handle.try_pop()
                returned = clock.tick()
                if value is None:
                    mine.append({"call": "pop", "outcome": "empty", "invoked": invoked, "returned": returned})
                    time.sleep(0)
                else:
                    mine.append(
                        {"call": "pop", "value": value, "outcome": "popped", "invoked": invoked, "returned": returned}
                    )
                    with delivered_lock:
                        delivered[0] += 1

    threads = [threading.Thread(target=produce, args=(p,)) for p in range(producers)]
    threads += [threading.Thread(target=consume, args=(c,)) for c in range(consumers)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    if failures:
        raise AssertionError(failures[0])
    return {"version": 1, "capacity": capacity, "refusal": "at-capacity", "participants": logs}


def _history_dir() -> Path:
    named = os.environ.get("SCE_QUEUE_HISTORY_DIR")
    if not named:
        raise unittest.SkipTest("SCE_QUEUE_HISTORY_DIR names where the histories are written; it is unset")
    path = Path(named)
    path.mkdir(parents=True, exist_ok=True)
    return path


def _write(directory: Path, name: str, document: dict[str, Any]) -> None:
    (directory / f"{name}.json").write_text(json.dumps(document, separators=(",", ":")) + "\n")


class TestQueueHistories(unittest.TestCase):
    def test_one_producer_one_consumer_runs_are_written_as_histories(self) -> None:
        directory = _history_dir()
        for capacity in (1, 2, 3, 8):
            for run in range(RUNS_PER_ONE_ONE_CAPACITY):
                document = _record_run(capacity, 1, 1, VALUES_PER_ONE_ONE_RUN)
                _write(directory, f"python_lock_n{capacity}_p1_c1_long_{run}", document)

    def test_many_participant_runs_are_written_as_histories(self) -> None:
        directory = _history_dir()
        for capacity, producers, consumers, per_producer in SHAPES:
            for run in range(RUNS_PER_SHAPE):
                document = _record_run(capacity, producers, consumers, per_producer)
                _write(directory, f"python_lock_n{capacity}_p{producers}_c{consumers}_{run}", document)


if __name__ == "__main__":
    unittest.main()
