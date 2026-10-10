# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Python arm of the `queue` kind's layer 1 (SCE Protocol-Synthesis RFC
# §synth-5-P): every scenario in tests/forge/conformance/queue_contract.json,
# the same ones the Rust, C++, Go and Kotlin arms run, against the runtime queue
# (one ring under one lock), and the properties of that runtime the scenarios do
# not reach: wrapping over many laps, the places of each side, the refusals of a
# constructor, the slot a pop clears, and that real threads lose nothing and
# reorder nothing between them.

from __future__ import annotations

import gc
import json
import threading
import time
import unittest
import weakref

import _sce_codegen
from sce_forge_runtime.queue import Consumer, Producer, PushStatus, Queue

CONTRACT = _sce_codegen.REPO_ROOT / "tests" / "forge" / "conformance" / "queue_contract.json"
DEADLINE_SECONDS = 120


class Tracked:
    """The element a scenario hands over."""

    def __init__(self, value: int) -> None:
        self.value = value


def _ring_places(scenario: dict) -> tuple[int, int]:
    """How many handles of each side a scenario's row allows. The scenarios hold
    one handle at a time, so one place each is enough; the SCQ rows are given the
    capacity so a row that is `many` is not the reason a scenario fails."""
    return (1, 1) if scenario["producers"] == "one" and scenario["consumers"] == "one" else (
        scenario["capacity"],
        scenario["capacity"],
    )


class TestContractScenarios(unittest.TestCase):
    def test_every_contract_scenario_holds(self) -> None:
        contract = json.loads(CONTRACT.read_text())
        self.assertEqual(1, contract["version"], "this arm reads version 1 of the contract")
        self.assertTrue(contract["scenarios"], "a contract with no scenarios checks nothing")
        for scenario in contract["scenarios"]:
            with self.subTest(scenario=scenario["id"]):
                if scenario["storage"] == "intrusive":
                    # A collected language has no element to link in place, and the
                    # allocation an intrusive list exists to avoid is the collector's.
                    # The generator refuses the row for Python by name
                    # (queue/storage-runtime-missing), so there is no runtime to run.
                    continue
                if scenario["storage"] == "segmented":
                    # Not lowered to Python yet; the generator refuses the row by
                    # name (queue/storage-runtime-missing), so there is no runtime.
                    continue
                self.assertEqual(
                    "bounded",
                    scenario["storage"],
                    f"the Python arm has no runtime for the row {scenario['storage']}",
                )
                producers, consumers = _ring_places(scenario)
                self._run(Queue(scenario["capacity"], producers, consumers), scenario)

    def _run(self, queue: Queue, scenario: dict) -> None:
        for index, step in enumerate(scenario["steps"]):
            context = f"{scenario['id']} step {index}"
            op = step["op"]
            if op == "capacity":
                self.assertEqual(step["expect"], queue.capacity, context)
            elif op == "push":
                with queue.producer() as producer:
                    status = producer.try_push(Tracked(step["value"]))
                want = {"ok": PushStatus.OK, "full": PushStatus.FULL}[step["expect"]]
                self.assertEqual(want, status, context)
            elif op == "pop":
                with queue.consumer() as consumer:
                    popped = consumer.try_pop()
                if step["expect"] == "empty":
                    self.assertIsNone(popped, f"{context}: expected the queue to be empty")
                else:
                    self.assertIsNotNone(popped, context)
                    self.assertEqual(step["expect"], popped.value, context)
            elif op == "destroy":
                # Destruction of held elements is a step for languages with
                # destructors. A Python queue owns no element the collector does
                # not also own; what Python owes instead is that a popped slot is
                # cleared, which test_a_popped_slot_is_cleared checks directly.
                return
            else:
                self.fail(f"{context}: unknown op {op}")


class TestRuntimeProperties(unittest.TestCase):
    def test_order_holds_across_many_laps(self) -> None:
        queue: Queue[int] = Queue(3, 1, 1)
        producer, consumer = queue.producer(), queue.consumer()
        assert producer is not None and consumer is not None
        pushed = popped = 0
        for _ in range(1000):
            while producer.try_push(pushed) is PushStatus.OK:
                pushed += 1
            while (value := consumer.try_pop()) is not None:
                self.assertEqual(popped, value)
                popped += 1
        self.assertEqual(pushed, popped)
        self.assertGreaterEqual(popped, 3000, "the run went round the ring many times")

    def test_exactly_the_capacity_fits_and_a_full_queue_keeps_what_it_holds(self) -> None:
        for capacity in (1, 2, 3, 5, 8):
            queue: Queue[int] = Queue(capacity, 2, 2)
            producer, consumer = queue.producer(), queue.consumer()
            assert producer is not None and consumer is not None
            accepted = 0
            while producer.try_push(accepted) is PushStatus.OK:
                accepted += 1
            self.assertEqual(capacity, accepted)
            self.assertEqual(PushStatus.FULL, producer.try_push(99))
            self.assertEqual(0, consumer.try_pop(), "a refused push disturbs nothing already inside")

    def test_a_side_hands_out_no_more_handles_than_it_has_places(self) -> None:
        queue: Queue[int] = Queue(2, 2, 1)
        first, second = queue.producer(), queue.producer()
        self.assertIsNotNone(first)
        self.assertIsNotNone(second)
        self.assertIsNone(queue.producer(), "a third producer has no place")
        assert first is not None and second is not None
        self.assertIsNone(first.try_clone())
        first.release()
        clone = second.try_clone()
        self.assertIsNotNone(clone, "a released place is free again")
        consumer = queue.consumer()
        self.assertIsNotNone(consumer)
        self.assertIsNone(queue.consumer(), "a `one` side has one place")

    def test_a_released_handle_is_not_usable(self) -> None:
        queue: Queue[int] = Queue(2, 1, 1)
        producer = queue.producer()
        assert producer is not None
        producer.release()
        producer.release()  # releasing twice gives nothing back twice
        self.assertIsNotNone(queue.producer(), "the place was given back exactly once")
        with self.assertRaises(RuntimeError):
            producer.try_push(1)

    def test_a_with_block_gives_the_place_back(self) -> None:
        queue: Queue[int] = Queue(2, 1, 1)
        with queue.producer() as producer:
            assert producer is not None
            self.assertIsNone(queue.producer())
        self.assertIsNotNone(queue.producer())

    def test_a_constructor_refuses_a_shape_it_cannot_run(self) -> None:
        for arguments in ((0, 1, 1), (-1, 1, 1), (2, 0, 1), (2, 1, 0)):
            with self.subTest(arguments=arguments), self.assertRaises(ValueError):
                Queue(*arguments)

    def test_a_popped_slot_is_cleared(self) -> None:
        queue: Queue[Tracked] = Queue(2, 1, 1)
        with queue.producer() as producer, queue.consumer() as consumer:
            assert producer is not None and consumer is not None
            element = Tracked(7)
            reference = weakref.ref(element)
            self.assertEqual(PushStatus.OK, producer.try_push(element))
            popped = consumer.try_pop()
            assert popped is not None
            self.assertEqual(7, popped.value)
            del element, popped
            gc.collect()
            self.assertIsNone(reference(), "the queue kept a reference to an element that has left it")


class TestThreads(unittest.TestCase):
    def test_threads_lose_nothing_and_reorder_nothing(self) -> None:
        producers, consumers, per_producer = 3, 3, 5000
        total = producers * per_producer
        queue: Queue[int] = Queue(8, producers, consumers)
        delivered = [0]
        delivered_lock = threading.Lock()
        failures: list[str] = []
        deadline = time.monotonic() + DEADLINE_SECONDS
        # Each consumer keeps the last value it saw from each producer: one
        # producer's elements reach one consumer in the order they were pushed.
        seen = [[-1] * producers for _ in range(consumers)]
        counts = [[0] * producers for _ in range(consumers)]

        def produce(p: int, handle: Producer[int]) -> None:
            with handle:
                for i in range(per_producer):
                    while handle.try_push(p * per_producer + i) is not PushStatus.OK:
                        if time.monotonic() > deadline:
                            failures.append("the elements did not all arrive before the deadline")
                            return
                        time.sleep(0)

        def consume(c: int, handle: Consumer[int]) -> None:
            with handle:
                while True:
                    with delivered_lock:
                        if delivered[0] >= total:
                            return
                    if time.monotonic() > deadline:
                        failures.append("the elements did not all arrive before the deadline")
                        return
                    value = handle.try_pop()
                    if value is None:
                        time.sleep(0)
                        continue
                    origin = value // per_producer
                    if value <= seen[c][origin]:
                        failures.append(f"consumer {c} saw {value} after {seen[c][origin]}")
                    seen[c][origin] = value
                    counts[c][origin] += 1
                    with delivered_lock:
                        delivered[0] += 1

        threads = []
        for p in range(producers):
            handle = queue.producer()
            assert handle is not None
            threads.append(threading.Thread(target=produce, args=(p, handle)))
        for c in range(consumers):
            handle = queue.consumer()
            assert handle is not None
            threads.append(threading.Thread(target=consume, args=(c, handle)))
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual([], failures[:5])
        for p in range(producers):
            self.assertEqual(
                per_producer,
                sum(row[p] for row in counts),
                f"every element of producer {p} arrived exactly once",
            )


if __name__ == "__main__":
    unittest.main()
