# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Python arm of the `broadcast-log` kind's runtime tests (ADR 0062): the
# unit tests of the Rust row (backends/rust/forge-runtime/src/broadcast_log.rs)
# ported to the runtime log (one ring under one lock), the properties of that
# runtime the Rust tests do not state (the refusals of a constructor, the
# handle's place, the horizon, a buffer a read leaves alone), a comparison with
# the sequential log of ADR 0062 section 2 over random operations, and real
# threads: one publisher and several readers that see whole entries in
# increasing order and account for every entry as read or lost.

from __future__ import annotations

import random
import threading
import time
import unittest

from sce_forge_runtime.broadcast_log import (
    HORIZON,
    PROGRESS,
    BroadcastLog,
    Empty,
    Item,
    Lagged,
    Read,
    Reader,
)
from sce_forge_runtime.queue import Progress

DEADLINE_SECONDS = 120
WORDS = 2
SLOTS = 3


def entry(n: int) -> list[int]:
    return [n] * WORDS


def make() -> BroadcastLog:
    return BroadcastLog(WORDS, SLOTS)


def read(log: BroadcastLog, reader: Reader) -> tuple[Read, list[int]]:
    out = [2**64 - 1] * WORDS
    return log.read(reader, out), out


class TestRowContract(unittest.TestCase):
    """The seven unit tests of the Rust row, on the same log shape."""

    def test_an_empty_log_is_empty_to_every_reader(self) -> None:
        log = make()
        reader = log.reader_oldest()
        self.assertEqual(Empty(), read(log, reader)[0])
        self.assertEqual(0, reader.cursor)
        self.assertEqual(0, log.head())

    def test_entries_come_back_in_order_with_their_words(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        reader = log.reader_oldest()
        self.assertEqual(0, publisher.publish(entry(10)))
        self.assertEqual(1, publisher.publish(entry(11)))
        self.assertEqual((Item(0), entry(10)), read(log, reader))
        self.assertEqual((Item(1), entry(11)), read(log, reader))
        self.assertEqual(Empty(), read(log, reader)[0])
        self.assertEqual(0, reader.lost)

    def test_the_window_is_exactly_the_retention(self) -> None:
        # 3 slots retain 2 entries.
        self.assertEqual(2, make().retention)
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        for n in range(3):
            publisher.publish(entry(n))
        # h = 3: the window is [1, 3), so a reader at 0 is one behind it.
        behind = log.reader_at(0)
        self.assertEqual(Lagged(1), read(log, behind)[0])
        self.assertEqual(1, behind.cursor)
        self.assertEqual(1, behind.lost)
        self.assertEqual((Item(1), entry(1)), read(log, behind))
        # A reader at the oldest retained entry lost nothing.
        oldest = log.reader_oldest()
        self.assertEqual(1, oldest.cursor)
        self.assertEqual((Item(1), entry(1)), read(log, oldest))

    def test_readers_do_not_share_a_cursor_or_a_loss(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        for n in range(5):
            publisher.publish(entry(n))
        a = log.reader_at(0)
        b = log.reader_at(3)
        self.assertEqual(Lagged(3), read(log, a)[0])
        self.assertEqual((Item(3), entry(3)), read(log, b))
        self.assertEqual((3, 0), (a.lost, b.lost))
        self.assertEqual(4, b.cursor)

    def test_a_reader_at_the_head_sees_only_what_comes_after(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        publisher.publish(entry(0))
        reader = log.reader_head()
        self.assertEqual(Empty(), read(log, reader)[0])
        publisher.publish(entry(1))
        self.assertEqual((Item(1), entry(1)), read(log, reader))
        # A position past the head is clamped to it.
        self.assertEqual(2, log.reader_at(99).cursor)

    def test_there_is_one_publisher_and_its_successor_continues_the_sequence(self) -> None:
        log = make()
        first = log.publisher()
        assert first is not None
        self.assertIsNone(log.publisher(), "a second publisher is refused")
        first.publish(entry(0))
        first.release()
        second = log.publisher()
        self.assertIsNotNone(second, "the place was freed")
        assert second is not None
        self.assertEqual(1, second.publish(entry(1)))

    def test_publishing_past_the_window_keeps_the_newest_entries(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        for n in range(100):
            publisher.publish(entry(n))
        reader = log.reader_oldest()
        self.assertEqual((Item(98), entry(98)), read(log, reader))
        self.assertEqual((Item(99), entry(99)), read(log, reader))
        self.assertEqual(Empty(), read(log, reader)[0])


class TestRuntimeProperties(unittest.TestCase):
    def test_this_row_gives_blocking(self) -> None:
        self.assertIs(Progress.BLOCKING, PROGRESS)

    def test_a_constructor_refuses_a_shape_it_cannot_run(self) -> None:
        for arguments in ((0, 3), (-1, 3), (2, 1), (2, 0), (2, -1)):
            with self.subTest(arguments=arguments), self.assertRaises(ValueError):
                BroadcastLog(*arguments)
        smallest = BroadcastLog(1, 2)
        self.assertEqual((1, 2, 1), (smallest.words, smallest.slots, smallest.retention))

    def test_retention_is_one_less_than_the_slots_for_every_slot_count(self) -> None:
        for slots in (2, 3, 4, 9):
            log = BroadcastLog(1, slots)
            publisher = log.publisher()
            assert publisher is not None
            for n in range(slots + 5):
                publisher.publish([n])
            reader = log.reader_at(0)
            out = [0]
            self.assertEqual(Lagged(slots + 5 - (slots - 1)), log.read(reader, out), f"slots={slots}")
            self.assertEqual(slots + 5 - (slots - 1), reader.cursor)
            for n in range(slots + 5 - (slots - 1), slots + 5):
                self.assertEqual(Item(n), log.read(reader, out))
                self.assertEqual([n], out)
            self.assertEqual(Empty(), log.read(reader, out))

    def test_a_reader_that_has_not_fallen_behind_by_a_whole_lap_is_never_told_it_lagged(self) -> None:
        # Publishing one entry and reading it, many times over many laps of a
        # small ring, never lags a reader that keeps up.
        log = BroadcastLog(1, 2)
        publisher = log.publisher()
        assert publisher is not None
        reader = log.reader_oldest()
        out = [0]
        for n in range(1000):
            publisher.publish([n])
            self.assertEqual(Item(n), log.read(reader, out))
            self.assertEqual([n], out)
            self.assertEqual(Empty(), log.read(reader, out))
        self.assertEqual(0, reader.lost)

    def test_a_read_that_does_not_return_an_item_leaves_the_buffer_alone(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        reader = log.reader_oldest()
        marker = [7, 9]
        self.assertEqual(Empty(), log.read(reader, marker))
        self.assertEqual([7, 9], marker)
        for n in range(5):
            publisher.publish(entry(n))
        behind = log.reader_at(0)
        self.assertEqual(Lagged(3), log.read(behind, marker))
        self.assertEqual([7, 9], marker)

    def test_a_reader_keeps_a_running_total_of_what_it_lost(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        reader = log.reader_oldest()
        skipped = 0
        for round_start in (0, 10, 40):
            for n in range(round_start, round_start + 6):
                publisher.publish(entry(n))
            # Only the last two of the six are retained: four skipped, then two read.
            result, _ = read(log, reader)
            assert isinstance(result, (Lagged, Item))
            if isinstance(result, Lagged):
                skipped += result.skipped
            self.assertEqual(skipped, reader.lost)
            while not isinstance(read(log, reader)[0], Empty):
                pass
            self.assertEqual(log.head(), reader.cursor)
        self.assertGreater(reader.lost, 0)

    def test_a_released_publisher_is_not_usable(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        publisher.release()
        publisher.release()  # releasing twice gives nothing back twice
        taken = log.publisher()
        self.assertIsNotNone(taken, "the place was given back exactly once")
        self.assertIsNone(log.publisher())
        with self.assertRaises(RuntimeError):
            publisher.publish(entry(1))

    def test_a_with_block_gives_the_place_back(self) -> None:
        log = make()
        with log.publisher() as publisher:
            assert publisher is not None
            self.assertIsNone(log.publisher())
        self.assertIsNotNone(log.publisher())

    def test_a_publish_that_is_refused_changes_nothing(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        publisher.publish(entry(0))
        for bad in ([1], [1, 2, 3], [1, -1], [1, 2**64]):
            with self.subTest(entry=bad), self.assertRaises(ValueError):
                publisher.publish(bad)
        self.assertEqual(1, log.head())
        self.assertEqual(1, publisher.publish(entry(1)), "the next sequence number is the next one")

    def test_a_publish_stores_a_copy_of_the_entry(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        mine = [5, 6]
        publisher.publish(mine)
        mine[0] = 99
        reader = log.reader_oldest()
        self.assertEqual((Item(0), [5, 6]), read(log, reader))

    def test_a_read_refuses_a_buffer_of_the_wrong_width_and_a_reader_of_another_log(self) -> None:
        log, other = make(), make()
        with self.assertRaises(ValueError):
            log.read(log.reader_oldest(), [0])
        with self.assertRaises(ValueError):
            log.read(other.reader_oldest(), [0, 0])

    def test_a_negative_position_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            make().reader_at(-1)

    def test_the_words_may_use_the_whole_64_bit_range(self) -> None:
        log = make()
        publisher = log.publisher()
        assert publisher is not None
        publisher.publish([0, 2**64 - 1])
        self.assertEqual((Item(0), [0, 2**64 - 1]), read(log, log.reader_oldest()))

    def test_the_log_stops_at_its_horizon(self) -> None:
        log = make()
        # The state a log is in after HORIZON - 1 entries, set directly: reaching
        # it by publishing would take longer than the run.
        log._head = HORIZON - 1
        publisher = log.publisher()
        assert publisher is not None
        self.assertEqual(HORIZON - 1, publisher.publish(entry(0)))
        self.assertEqual(HORIZON, log.head())
        with self.assertRaises(OverflowError):
            publisher.publish(entry(1))
        self.assertEqual(HORIZON, log.head(), "the refused publish changed nothing")

    def test_the_horizon_is_the_rust_rows(self) -> None:
        self.assertEqual((2**64 - 3) // 2, HORIZON)
        self.assertLess(2 * HORIZON + 2, 2**64)


class TestAgainstTheSequentialLog(unittest.TestCase):
    """Random operations on the log and on the sequential log of ADR 0062
    section 2, written out here as a list and a count."""

    def test_every_result_is_what_the_sequential_log_gives(self) -> None:
        for slots in (2, 3, 4, 7):
            for seed in range(20):
                with self.subTest(slots=slots, seed=seed):
                    self._run(slots, random.Random(seed * 1000 + slots))

    def _run(self, slots: int, rng: random.Random) -> None:
        retention = slots - 1
        log = BroadcastLog(2, slots)
        publisher = log.publisher()
        assert publisher is not None
        entries: list[list[int]] = []
        readers: list[Reader] = []
        cursors: list[int] = []
        losses: list[int] = []
        for step in range(400):
            choice = rng.randrange(10)
            h = len(entries)
            if choice < 3:
                value = [step, step + 1]
                self.assertEqual(h, publisher.publish(value))
                entries.append(value)
            elif choice == 3 and len(readers) < 4:
                kind = rng.randrange(3)
                if kind == 0:
                    reader, cursor = log.reader_oldest(), max(0, h - retention)
                elif kind == 1:
                    reader, cursor = log.reader_head(), h
                else:
                    at = rng.randrange(0, h + 3)
                    reader, cursor = log.reader_at(at), min(at, h)
                self.assertEqual(cursor, reader.cursor)
                readers.append(reader)
                cursors.append(cursor)
                losses.append(0)
            elif readers:
                which = rng.randrange(len(readers))
                out = [0, 0]
                got = log.read(readers[which], out)
                r = cursors[which]
                if r == h:
                    self.assertEqual(Empty(), got)
                elif r >= h - retention:
                    self.assertEqual(Item(r), got)
                    self.assertEqual(entries[r], out)
                    cursors[which] = r + 1
                else:
                    skipped = (h - retention) - r
                    self.assertEqual(Lagged(skipped), got)
                    cursors[which] = h - retention
                    losses[which] += skipped
                self.assertEqual(cursors[which], readers[which].cursor)
                self.assertEqual(losses[which], readers[which].lost)
            self.assertEqual(len(entries), log.head())


class TestThreads(unittest.TestCase):
    def test_readers_see_whole_entries_in_increasing_order_and_account_for_every_entry(self) -> None:
        readers_count, total, words, slots = 4, 20000, 4, 5
        log = BroadcastLog(words, slots)
        failures: list[str] = []
        done = threading.Event()
        deadline = time.monotonic() + DEADLINE_SECONDS
        # Per reader: the entries it read, the entries it was told it lost.
        read_counts = [0] * readers_count
        lost_counts = [0] * readers_count
        handles = [log.reader_oldest() for _ in range(readers_count)]

        def publish() -> None:
            publisher = log.publisher()
            assert publisher is not None
            with publisher:
                for n in range(total):
                    if time.monotonic() > deadline:
                        failures.append("the publisher did not finish before the deadline")
                        break
                    publisher.publish([n] * words)
                    if n % 64 == 0:
                        time.sleep(0)
            done.set()

        def consume(c: int) -> None:
            handle = handles[c]
            last = -1
            out = [0] * words
            while True:
                finished = done.is_set()
                if time.monotonic() > deadline:
                    failures.append(f"reader {c} did not finish before the deadline")
                    return
                result = log.read(handle, out)
                if isinstance(result, Item):
                    read_counts[c] += 1
                    if any(word != result.seq for word in out):
                        failures.append(f"reader {c} got a torn entry {out} for sequence {result.seq}")
                    if result.seq <= last:
                        failures.append(f"reader {c} saw {result.seq} after {last}")
                    last = result.seq
                elif isinstance(result, Lagged):
                    lost_counts[c] += result.skipped
                    if result.skipped < 1:
                        failures.append(f"reader {c} was told it lagged by {result.skipped}")
                elif finished:
                    return
                else:
                    time.sleep(0)

        threads = [threading.Thread(target=publish)]
        threads += [threading.Thread(target=consume, args=(c,)) for c in range(readers_count)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual([], failures[:5])
        self.assertEqual(total, log.head())
        for c in range(readers_count):
            self.assertEqual(lost_counts[c], handles[c].lost, f"reader {c}'s own total is the sum of what it was told")
            self.assertEqual(total, read_counts[c] + handles[c].lost, f"reader {c} read or lost every entry once")
            self.assertEqual(total, handles[c].cursor)


if __name__ == "__main__":
    unittest.main()
