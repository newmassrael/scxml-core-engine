// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The C++ arm of the broadcast-log kind's narrow row (ADR 0062, "Stamp width"), at the width
// a row keeps: sce/forge/broadcast_log_narrow.h with `kNarrowStampBits`, 32. The wrap cases
// are broadcast_log_narrow_runtime_test.cpp's, which names a stamp of 3 bits because a stamp
// of 32 cannot reach them; this file holds what only the full width shows (the arithmetic at
// the top of a 32-bit space, a word that keeps all its bits, a lag counted one lap on in 64
// bits), the slot counts the class accepts, and real threads: one publisher and several
// readers, where every entry a reader is handed must be whole and the entries it is handed
// and the entries it is told it missed must add up to every entry published.
//
// An entry is three words, all of them the sequence number plus one: a copy that mixes two
// entries shows as words that differ, and a copy of a slot that nothing was written to shows
// as a word of zero, which no entry has.

#include "sce/forge/broadcast_log_narrow.h"

#include <array>
#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <thread>
#include <type_traits>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

namespace blog = ::SCE::Forge::Broadcast;

using Arith = blog::detail::StampArith<32>;
// Four slots retain three entries.
using Log = blog::NarrowBroadcastLog<3, 4>;
using Entry = Log::Entry;

Entry entry_of(std::uint32_t seq) {
    return Entry{seq + 1, seq + 1, seq + 1};
}

bool is_whole_entry(const Entry &words, std::uint64_t seq) {
    const auto want = static_cast<std::uint32_t>(seq + 1);
    return words[0] == want && words[1] == want && words[2] == want;
}

// ─── The row's width ───

void the_row_keeps_a_stamp_of_32_bits() {
    static_assert(blog::kNarrowStampBits == 32, "a row keeps a stamp of 32 bits");
    static_assert(std::is_same<Log, blog::BroadcastLog<3, 4, blog::Narrow<32>>>::value,
                  "the row's log is the log over the lane of the width a row keeps");
    static_assert(std::is_same<blog::Narrow<32>::Raw, std::uint32_t>::value, "a word of the row is 32 bits");
    static_assert(std::is_same<Entry, std::array<std::uint32_t, 3>>::value, "an entry is 32-bit words");
    static_assert(Arith::kMask == UINT32_MAX, "the stamp is the whole word");
    static_assert(Arith::kSpace == UINT64_C(0x100000000), "2^32 values");
    static_assert(Arith::kHalf == UINT64_C(0x80000000), "a stamp tells 2^31 entries of one slot apart");
    static_assert(blog::Narrow<32>::kUnwritten == 1, "an unwritten slot is stamped 1");
    CHECK(Arith::kHalf == UINT64_C(0x80000000), "half of the stamp space is 2^31");
}

// done: entries 2^31 - 1 and 2^32 - 1 (half a stamp space apart) have the same stamps.
// 2 * (2^31 - 1) + 2 = 2^32 and 2 * (2^32 - 1) + 2 = 2^33, both 0 modulo 2^32. busy:
// 2 * (2^31 - 1) + 1 = 2^32 - 1 and 2 * (2^32 - 1) + 1 = 2^33 - 1, both 2^32 - 1.
void a_stamp_tells_apart_half_of_a_32_bit_space() {
    CHECK(Arith::done(0x7FFFFFFF) == 0, "done(2^31 - 1)");
    CHECK(Arith::done(0xFFFFFFFF) == 0, "done(2^32 - 1)");
    CHECK(Arith::done(0) == 2, "done(0)");
    CHECK(Arith::busy(0x7FFFFFFF) == 0xFFFFFFFF, "busy(2^31 - 1)");
    CHECK(Arith::busy(0xFFFFFFFF) == 0xFFFFFFFF, "busy(2^32 - 1)");
    CHECK(Arith::busy(0) == 1, "busy(0)");
}

// A lag counted from a head that wrapped is one lap on, and a lap is 2^32 entries: the count
// does not fit 32 bits, which is why `skipped` and `lost` are 64.
void a_lag_from_a_wrapped_head_is_counted_a_lap_on_in_64_bits() {
    // 1 + 2^32 behind, less the retention 3: the head says the reader is 1 behind, which is
    // inside the window of 4 slots and so cannot be what a lag is answered from.
    CHECK(Arith::missed(1, 4, 3) == UINT64_C(4294967294), "2^32 + 1 - 3");
    // (10 + 2^32 - 2) mod 2^32 is the residue of the oldest retained entry, 11 - 3 = 8.
    CHECK(Arith::advance(10, Arith::missed(1, 4, 3)) == 8, "the cursor lands on the oldest retained entry");
    // A distance of 3 is inside the window of 4 slots as well; the lap added to it is the
    // fewest it can be, 2^32 + 3 - 3.
    CHECK(Arith::missed(3, 4, 3) == UINT64_C(4294967296), "2^32 itself, which 32 bits cannot hold");
    CHECK(Arith::advance(10, Arith::missed(3, 4, 3)) == 10, "and the cursor is where it was");
    // 4 behind is outside the window of 3 and is taken as it is: 4 - 3.
    CHECK(Arith::missed(4, 4, 3) == 1, "a distance of the slot count is a lag of 1");
    CHECK(Arith::advance(10, Arith::missed(4, 4, 3)) == 11, "the cursor moves by it");
}

void a_word_keeps_all_of_its_32_bits() {
    blog::NarrowBroadcastLog<2, 4> log;
    auto publisher = log.split();
    CHECK(publisher.has_value(), "the publisher");
    const blog::NarrowBroadcastLog<2, 4>::Entry entry{UINT32_C(0xDEADBEEF), UINT32_C(0x01020304)};
    CHECK(publisher->publish(entry) == 0, "entry 0");
    blog::Reader reader = log.reader_oldest();
    blog::NarrowBroadcastLog<2, 4>::Entry out{0, 0};
    CHECK(log.read(reader, out) == blog::Read::item(0), "it is read back");
    CHECK(out[0] == UINT32_C(0xDEADBEEF) && out[1] == UINT32_C(0x01020304), "each word keeps all of its 32 bits");
}

// ─── The slot counts ───

// What the class accepts at 32 bits: a power of two of at least two slots, at most 2^31. A
// log of another size does not compile, so the predicate the class asserts is what is held.
void the_class_accepts_a_power_of_two_of_slots_and_refuses_the_rest() {
    static_assert(!Arith::slots_fit(1), "one slot would retain nothing");
    static_assert(!Arith::slots_fit(0), "no slot would retain nothing");
    static_assert(!Arith::slots_fit(3), "3 slots do not divide 2^32");
    static_assert(!Arith::slots_fit(7), "7 slots do not divide 2^32");
    static_assert(!Arith::slots_fit(UINT64_C(0xC0000000)), "a slot count that is not a power of two");
    static_assert(!Arith::slots_fit(UINT64_C(0x100000000)), "2^32 slots are more than half the stamp space");
    static_assert(Arith::slots_fit(2), "two slots retain one entry");
    static_assert(Arith::slots_fit(8), "eight slots retain seven");
    static_assert(Arith::slots_fit(UINT64_C(0x80000000)), "half the stamp space is the most");
    static_assert(blog::NarrowBroadcastLog<1, 2>::kRetention == 1, "two slots retain one");
    static_assert(blog::NarrowBroadcastLog<1, 8>::kRetention == 7, "eight slots retain seven");
    CHECK(Arith::slots_fit(2) && !Arith::slots_fit(3), "the predicate is the one the class asserts");
}

// A new log is stamped unwritten and empty: its oldest reader is at the head.
void a_new_log_is_empty_and_its_oldest_reader_is_at_the_head() {
    Log log;
    blog::Reader oldest = log.reader_oldest();
    Entry out{};
    CHECK(log.head() == 0, "nothing is published");
    CHECK(oldest.cursor() == 0 && log.read(oldest, out) == blog::Read::empty(),
          "the oldest reader of a new log is at its head");
}

// ─── The deferred reader ───

// A deferred reader of the 32-bit row: no cursor and not bound until its first read, which binds
// the oldest retained entry of the head it loads, as the oldest reader of the same moment would,
// and tells a reader that waited nothing. Four slots retain three.
void a_deferred_reader_binds_at_its_first_read_in_the_32_bit_row() {
    Log log;
    auto publisher = log.split();
    CHECK(publisher.has_value(), "the publisher");
    blog::Reader deferred = log.reader_deferred();
    CHECK(!deferred.is_bound(), "not bound before its first read");
    CHECK(deferred.cursor() == 0 && deferred.lost() == 0, "no cursor reads as 0, with no loss");
    Entry out{};
    CHECK(log.read(deferred, out) == blog::Read::empty() && deferred.is_bound() && deferred.cursor() == 0,
          "an empty log is empty, and the read bound the reader at the start");
    // A young log, last slot never written: three entries published, the oldest is 0.
    for (std::uint32_t n = 0; n < 3; ++n) {
        CHECK(publisher->publish(entry_of(n)) == n, "publish takes the next sequence number");
    }
    deferred = log.reader_deferred();
    CHECK(log.read(deferred, out) == blog::Read::item(0) && is_whole_entry(out, 0),
          "a young log binds to entry 0 by its last slot");
    // Ten more: the head is 13, and a reader that waited is told of nothing, where the oldest
    // reader opened before them is told it missed 13 - 3 = 10.
    blog::Reader eager = log.reader_oldest();
    for (std::uint32_t n = 3; n < 13; ++n) {
        CHECK(publisher->publish(entry_of(n)) == n, "publish takes the next sequence number");
    }
    deferred = log.reader_deferred();
    for (std::uint32_t n = 10; n < 13; ++n) {
        CHECK(log.read(deferred, out) == blog::Read::item(n) && is_whole_entry(out, n),
              "the deferred reader reads the oldest three in order");
    }
    CHECK(deferred.lost() == 0, "with nothing lost");
    CHECK(log.read(deferred, out) == blog::Read::empty(), "then the head");
    CHECK(log.read(eager, out) == blog::Read::lagged(10), "the eager reader is told it missed 10");
}

// ─── The publisher ───

// The three ways to take the place are one place: each refuses the others while it is held, a
// swap-less target uses the ownership forms, and the successor continues.
void the_publishers_forms_are_one_place() {
    blog::NarrowBroadcastLog<3, 2> log;
    {
        auto first = log.publisher();
        CHECK(first.has_value(), "the exchange takes the place");
        CHECK(!log.publisher().has_value(), "a second exchange is refused");
        CHECK(!log.split().has_value(), "and so is the ownership form");
        CHECK(first->publish(entry_of(0)) == 0, "entry 0");
    }
    {
        auto second = log.split();
        CHECK(second.has_value(), "the ownership form takes the freed place");
        CHECK(!log.publisher().has_value(), "which the exchange then finds taken");
        CHECK(second->publish(entry_of(1)) == 1, "the sequence continues");
    }
    {
        auto unchecked = log.publisher_unchecked();
        CHECK(unchecked.publish(entry_of(2)) == 2, "the unchecked publisher continues it too");
        CHECK(!log.split().has_value(), "and holds the place against the ownership form");
    }
    CHECK(log.head() == 3, "three entries are published");
}

// ─── Real threads ───

constexpr std::uint64_t kStressEntries = 200000;
constexpr int kStressReaders = 3;

struct ReaderResult {
    std::uint64_t items = 0;
    std::uint64_t lost = 0;
    std::uint64_t torn = 0;
    std::uint64_t out_of_order = 0;
    std::uint64_t bad_lag = 0;
    bool timed_out = false;
};

template <std::size_t Slots> void one_publisher_and_several_readers_see_whole_entries() {
    using StressLog = blog::NarrowBroadcastLog<3, Slots>;
    StressLog log;
    std::vector<ReaderResult> results(kStressReaders);
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(120);
    std::atomic<bool> publisher_failed{false};

    std::vector<std::thread> threads;
    for (int i = 0; i < kStressReaders; ++i) {
        threads.emplace_back([&, i] {
            ReaderResult &result = results[static_cast<std::size_t>(i)];
            blog::Reader reader = log.reader_at(0);
            std::uint64_t last_item_plus_one = 0;
            typename StressLog::Entry out{};
            // The count does not wrap in a run of this length, so a cursor is the count.
            while (reader.cursor() < kStressEntries) {
                const std::uint64_t before = reader.cursor();
                const std::uint64_t lost_before = reader.lost();
                const blog::Read read = log.read(reader, out);
                switch (read.kind) {
                case blog::ReadKind::Item:
                    ++result.items;
                    if (read.seq != before || reader.cursor() != before + 1 || reader.lost() != lost_before ||
                        out != typename StressLog::Entry{static_cast<std::uint32_t>(read.seq + 1),
                                                         static_cast<std::uint32_t>(read.seq + 1),
                                                         static_cast<std::uint32_t>(read.seq + 1)}) {
                        ++result.torn;
                    }
                    if (read.seq < last_item_plus_one) {
                        ++result.out_of_order;
                    }
                    last_item_plus_one = read.seq + 1;
                    break;
                case blog::ReadKind::Lagged:
                    if (read.skipped < 1 || reader.cursor() != before + read.skipped ||
                        reader.lost() != lost_before + read.skipped) {
                        ++result.bad_lag;
                    }
                    break;
                case blog::ReadKind::Empty:
                    if (reader.cursor() != before) {
                        ++result.bad_lag;
                    }
                    if (std::chrono::steady_clock::now() > deadline) {
                        result.timed_out = true;
                        return;
                    }
                    std::this_thread::yield();
                    break;
                }
            }
            result.lost = reader.lost();
        });
    }
    threads.emplace_back([&] {
        auto publisher = log.publisher();
        if (!publisher) {
            publisher_failed = true;
            return;
        }
        for (std::uint64_t n = 0; n < kStressEntries; ++n) {
            const auto word = static_cast<std::uint32_t>(n + 1);
            if (publisher->publish(typename StressLog::Entry{word, word, word}) != n) {
                publisher_failed = true;
                return;
            }
            // Now and then give the readers the processor, so that they are sometimes
            // ahead of the publisher and sometimes far behind it.
            if (n % 1024 == 1023) {
                std::this_thread::yield();
            }
        }
    });
    for (std::thread &t : threads) {
        t.join();
    }

    CHECK(!publisher_failed, "the publisher took its place and every sequence number in order");
    CHECK(log.head() == kStressEntries, "every entry was published");
    for (const ReaderResult &result : results) {
        CHECK(!result.timed_out, "a reader finished before the deadline");
        CHECK(result.torn == 0, "every item is one whole entry at the reader's cursor");
        CHECK(result.out_of_order == 0, "the sequence numbers of a reader's items strictly increase");
        CHECK(result.bad_lag == 0, "a lag skips at least one entry and moves the cursor exactly by it");
        // Every sequence number below the total was handed over or counted lost.
        CHECK(result.items + result.lost == kStressEntries, "items and lost entries make every entry published");
    }
}

void real_threads_see_whole_entries_and_lose_only_what_they_are_told() {
    one_publisher_and_several_readers_see_whole_entries<2>();
    one_publisher_and_several_readers_see_whole_entries<4>();
    one_publisher_and_several_readers_see_whole_entries<8>();
}

}  // namespace

int main() {
    the_row_keeps_a_stamp_of_32_bits();
    a_stamp_tells_apart_half_of_a_32_bit_space();
    a_lag_from_a_wrapped_head_is_counted_a_lap_on_in_64_bits();
    a_word_keeps_all_of_its_32_bits();
    the_class_accepts_a_power_of_two_of_slots_and_refuses_the_rest();
    a_new_log_is_empty_and_its_oldest_reader_is_at_the_head();
    a_deferred_reader_binds_at_its_first_read_in_the_32_bit_row();
    the_publishers_forms_are_one_place();
    real_threads_see_whole_entries_and_lose_only_what_they_are_told();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_narrow_runtime_32_test: all checks passed");
    return 0;
}
