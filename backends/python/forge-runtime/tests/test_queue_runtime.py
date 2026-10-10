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
from sce_forge_runtime.queue import (
    Consumer,
    Producer,
    Progress,
    PushStatus,
    Queue,
    SegmentedQueue,
)

CONTRACT = _sce_codegen.REPO_ROOT / "tests" / "forge" / "conformance" / "queue_contract.json"
DEADLINE_SECONDS = 120


class Tracked:
    """The element a scenario hands over."""

    def __init__(self, value: int) -> None:
        self.value = value


class BudgetAllocator:
    """The allocator the segmented queue's tests inject: it gives at most `limit`
    segments at once (no limit when `limit` is None), counting the ones it has out
    so a test can say that the queue gave every segment back. `progress` is what a
    test says it gives, so a test can inject one that gives less than a queue's
    document declared for it. The queue calls it under its own lock."""

    def __init__(self, limit: int | None = None, progress: Progress = Progress.LOCK_FREE) -> None:
        self.limit = limit
        self.progress = progress
        self.live = 0
        self.granted = 0

    def allocate(self) -> bool:
        if self.limit is not None and self.live >= self.limit:
            return False
        self.live += 1
        self.granted += 1
        return True

    def deallocate(self) -> None:
        self.live -= 1


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
                    # Every cardinality is one list of segments under one lock. The
                    # allocator gives the scenario's `segments` at once, and all it
                    # is asked for when the scenario names none.
                    allocator = BudgetAllocator(scenario.get("segments"))
                    queue = SegmentedQueue.create(scenario["segment"], allocator, Progress.BLOCKING, 1, 1)
                    self.assertIsNotNone(queue, "the allocator gives the first segment")
                    self._run(queue, scenario)
                    continue
                self.assertEqual(
                    "bounded",
                    scenario["storage"],
                    f"the Python arm has no runtime for the row {scenario['storage']}",
                )
                producers, consumers = _ring_places(scenario)
                self._run(Queue(scenario["capacity"], producers, consumers), scenario)

    def _run(self, queue: Queue | SegmentedQueue, scenario: dict) -> None:
        for index, step in enumerate(scenario["steps"]):
            context = f"{scenario['id']} step {index}"
            op = step["op"]
            if op == "capacity":
                self.assertEqual(step["expect"], queue.capacity, context)
            elif op == "push":
                with queue.producer() as producer:
                    status = producer.try_push(Tracked(step["value"]))
                want = {
                    "ok": PushStatus.OK,
                    "full": PushStatus.FULL,
                    "out_of_memory": PushStatus.OUT_OF_MEMORY,
                }[step["expect"]]
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


class TestSegmentedRuntime(unittest.TestCase):
    def test_an_allocator_that_gives_less_than_the_document_declared_is_refused(self) -> None:
        allocator = BudgetAllocator(progress=Progress.LOCK_FREE)
        for required in (Progress.BLOCKING, Progress.LOCK_FREE):
            self.assertIsNotNone(SegmentedQueue.create(2, allocator, required, 1, 1))
        with self.assertRaises(ValueError):
            SegmentedQueue.create(2, allocator, Progress.WAIT_FREE, 1, 1)
        self.assertEqual(2, allocator.live, "the refused queue asked for no segment")

    def test_a_shape_it_cannot_run_is_refused_before_the_allocator_is_asked(self) -> None:
        allocator = BudgetAllocator()
        for arguments in ((0, 1, 1), (2, 0, 1), (2, 1, 0)):
            with self.subTest(arguments=arguments), self.assertRaises(ValueError):
                SegmentedQueue.create(arguments[0], allocator, Progress.BLOCKING, arguments[1], arguments[2])
        self.assertEqual(0, allocator.granted, "a bad shape is not an allocator's refusal")

    def test_a_queue_whose_first_segment_is_refused_is_not_built(self) -> None:
        self.assertIsNone(SegmentedQueue.create(2, BudgetAllocator(0), Progress.BLOCKING, 1, 1))

    def test_each_segment_goes_back_as_the_consumer_leaves_it(self) -> None:
        allocator = BudgetAllocator()
        queue = SegmentedQueue.create(3, allocator, Progress.BLOCKING, 1, 1)
        assert queue is not None
        with queue.producer() as producer, queue.consumer() as consumer:
            assert producer is not None and consumer is not None
            for i in range(4 * 3 + 1):
                self.assertEqual(PushStatus.OK, producer.try_push(i))
            self.assertEqual(5, allocator.live, "thirteen elements of three are in five segments")
            for i in range(4 * 3 + 1):
                self.assertEqual(i, consumer.try_pop())
            self.assertIsNone(consumer.try_pop())
            self.assertEqual(1, allocator.live, "the consumer gave back every segment it left")

    def test_the_queue_holds_what_the_allocator_allows_and_is_room_again_once_a_segment_goes_back(self) -> None:
        queue = SegmentedQueue.create(2, BudgetAllocator(2), Progress.BLOCKING, 1, 1)
        assert queue is not None
        with queue.producer() as producer, queue.consumer() as consumer:
            assert producer is not None and consumer is not None
            for i in range(4):
                self.assertEqual(PushStatus.OK, producer.try_push(i))
            for _ in range(2):
                self.assertEqual(PushStatus.OUT_OF_MEMORY, producer.try_push(4), "a refusal changes nothing")
            for i in range(3):
                self.assertEqual(i, consumer.try_pop())
            self.assertEqual(PushStatus.OK, producer.try_push(4), "the segment the consumer left is room again")
            self.assertEqual(3, consumer.try_pop())
            self.assertEqual(4, consumer.try_pop())
            self.assertIsNone(consumer.try_pop())

    def test_a_side_hands_out_no_more_handles_than_it_has_places(self) -> None:
        queue = SegmentedQueue.create(2, BudgetAllocator(), Progress.BLOCKING, 2, 1)
        assert queue is not None
        first, second = queue.producer(), queue.producer()
        assert first is not None and second is not None
        self.assertIsNone(queue.producer(), "a third producer has no place")
        self.assertIsNone(first.try_clone())
        first.release()
        self.assertIsNotNone(second.try_clone(), "a released place is free again")
        self.assertIsNotNone(queue.consumer())
        self.assertIsNone(queue.consumer(), "a `one` side has one place")

    def test_a_released_handle_is_not_usable(self) -> None:
        queue = SegmentedQueue.create(2, BudgetAllocator(), Progress.BLOCKING, 1, 1)
        assert queue is not None
        producer = queue.producer()
        assert producer is not None
        producer.release()
        producer.release()  # releasing twice gives nothing back twice
        self.assertIsNotNone(queue.producer(), "the place was given back exactly once")
        with self.assertRaises(RuntimeError):
            producer.try_push(1)

    def test_a_popped_slot_is_cleared(self) -> None:
        queue = SegmentedQueue.create(2, BudgetAllocator(), Progress.BLOCKING, 1, 1)
        assert queue is not None
        with queue.producer() as producer, queue.consumer() as consumer:
            assert producer is not None and consumer is not None
            element = Tracked(7)
            reference = weakref.ref(element)
            self.assertEqual(PushStatus.OK, producer.try_push(element))
            popped = consumer.try_pop()
            assert popped is not None
            del element, popped
            gc.collect()
            self.assertIsNone(reference(), "the queue kept a reference to an element that has left it")


class TestThreads(unittest.TestCase):
    def test_segmented_threads_lose_nothing_and_reorder_nothing(self) -> None:
        # The allocator allows only a few segments at once, so producers meet
        # refusals and wait for the consumers to give segments back.
        producers, consumers, per_producer = 3, 3, 3000
        total = producers * per_producer
        allocator = BudgetAllocator(6)
        queue = SegmentedQueue.create(4, allocator, Progress.BLOCKING, producers, consumers)
        assert queue is not None
        delivered = [0]
        delivered_lock = threading.Lock()
        failures: list[str] = []
        deadline = time.monotonic() + DEADLINE_SECONDS
        seen = [[-1] * producers for _ in range(consumers)]
        counts = [[0] * producers for _ in range(consumers)]

        def produce(p: int, handle: object) -> None:
            with handle:  # type: ignore[attr-defined]
                for i in range(per_producer):
                    while handle.try_push(p * per_producer + i) is not PushStatus.OK:  # type: ignore[attr-defined]
                        if time.monotonic() > deadline:
                            failures.append("the elements did not all arrive before the deadline")
                            return
                        time.sleep(0)

        def consume(c: int, handle: object) -> None:
            with handle:  # type: ignore[attr-defined]
                while True:
                    with delivered_lock:
                        if delivered[0] >= total:
                            return
                    if time.monotonic() > deadline:
                        failures.append("the elements did not all arrive before the deadline")
                        return
                    value = handle.try_pop()  # type: ignore[attr-defined]
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
            threads.append(threading.Thread(target=produce, args=(p, queue.producer())))
        for c in range(consumers):
            threads.append(threading.Thread(target=consume, args=(c, queue.consumer())))
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual([], failures[:5])
        for p in range(producers):
            self.assertEqual(per_producer, sum(row[p] for row in counts), f"every element of producer {p} arrived once")
        self.assertTrue(1 <= allocator.live <= 6, f"the allocator's ceiling held and the rest went back: {allocator.live}")

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
