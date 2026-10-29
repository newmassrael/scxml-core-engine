// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/broadcast_log.h — the C++ arm of the `broadcast-log` kind's layer 1
// (RFC §synth-5-S, ADR 0062): the unit tests of the Rust row
// (backends/rust/forge-runtime/src/broadcast_log.rs) run against the C++ runtime,
// the properties the C++ row adds (a log in static storage is
// constant-initialised, an oversized publish is refused), and real threads: one
// publisher and several readers, where every entry a reader is handed must be
// whole and the entries it is handed and the entries it is told it missed must
// add up to every entry published.
//
// An entry is three words, all of them the sequence number plus one: a copy that
// mixes two entries shows as words that differ, and a copy of a slot that nothing
// was written to shows as a word of zero, which no entry has.
//
// The thread tests are also inputs of the sanitizer lanes (scripts/build_tsan.sh).

#include "sce/forge/broadcast_log.h"

#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <thread>
#include <utility>
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

// Three slots retain two entries, as the Rust row's unit tests do.
using Log = blog::BroadcastLog<3, 3>;
using Entry = Log::Entry;

Entry entry_of(std::uint64_t seq) {
    return Entry{seq + 1, seq + 1, seq + 1};
}

struct Got {
    blog::Read read;
    Entry words;
};

// One read into a buffer of the sentinel, so a read that wrote nothing is seen.
Got read_into(const Log &log, blog::Reader &reader) {
    Got got{blog::Read::empty(), Entry{UINT64_MAX, UINT64_MAX, UINT64_MAX}};
    got.read = log.read(reader, got.words);
    return got;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━ The unit tests of the Rust row ━━━━━━━━━━━━━━━━━━━━━━━━

void an_empty_log_is_empty_to_every_reader() {
    Log log;
    blog::Reader reader = log.reader_oldest();
    CHECK(read_into(log, reader).read == blog::Read::empty(), "an empty log reads as empty");
    CHECK(reader.cursor() == 0, "an empty read moves nothing");
    CHECK(log.head() == 0, "an empty log has published nothing");
    CHECK(Log::kRetention == 2, "the retention is one less than the slots");
}

void entries_come_back_in_order_with_their_words() {
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    blog::Reader reader = log.reader_oldest();
    CHECK(publisher->publish(entry_of(0)) == 0, "publish 0 takes sequence number 0");
    CHECK(publisher->publish(entry_of(1)) == 1, "publish 1 takes sequence number 1");
    Got first = read_into(log, reader);
    CHECK(first.read == blog::Read::item(0) && first.words == entry_of(0), "entry 0 comes back with its words");
    Got second = read_into(log, reader);
    CHECK(second.read == blog::Read::item(1) && second.words == entry_of(1), "entry 1 comes back with its words");
    CHECK(read_into(log, reader).read == blog::Read::empty(), "then the head");
    CHECK(reader.lost() == 0, "a reader that kept up lost nothing");
    CHECK(reader.cursor() == 2, "the cursor moved past both entries");
}

void the_window_is_exactly_the_retention() {
    Log log;
    auto publisher = log.publisher();
    for (std::uint64_t n = 0; n < 3; ++n) {
        publisher->publish(entry_of(n));
    }
    // h = 3: the window is [1, 3), so a reader at 0 is one behind it.
    blog::Reader behind = log.reader_at(0);
    CHECK(read_into(log, behind).read == blog::Read::lagged(1), "a reader at 0 is one behind the window");
    CHECK(behind.cursor() == 1, "a lag moves the cursor to the oldest retained entry");
    CHECK(behind.lost() == 1, "a lag adds what it skipped to the reader's total");
    Got after = read_into(log, behind);
    CHECK(after.read == blog::Read::item(1) && after.words == entry_of(1), "after a lag the reader reads the oldest");
    // A reader at the oldest retained entry lost nothing.
    blog::Reader oldest = log.reader_oldest();
    CHECK(oldest.cursor() == 1, "the oldest reader starts at the start of the window");
    Got first = read_into(log, oldest);
    CHECK(first.read == blog::Read::item(1) && first.words == entry_of(1), "a reader at the oldest reads it");
}

void readers_do_not_share_a_cursor_or_a_loss() {
    Log log;
    auto publisher = log.publisher();
    for (std::uint64_t n = 0; n < 5; ++n) {
        publisher->publish(entry_of(n));
    }
    blog::Reader a = log.reader_at(0);
    blog::Reader b = log.reader_at(3);
    CHECK(read_into(log, a).read == blog::Read::lagged(3), "a reader at 0 of 5 is 3 behind a window of 2");
    Got got = read_into(log, b);
    CHECK(got.read == blog::Read::item(3) && got.words == entry_of(3), "the other reader reads entry 3");
    CHECK(a.lost() == 3 && b.lost() == 0, "each reader has its own loss");
    CHECK(b.cursor() == 4, "and its own cursor");
}

void a_reader_at_the_head_sees_only_what_comes_after() {
    Log log;
    auto publisher = log.publisher();
    publisher->publish(entry_of(0));
    blog::Reader reader = log.reader_head();
    CHECK(read_into(log, reader).read == blog::Read::empty(), "a reader at the head sees nothing yet");
    publisher->publish(entry_of(1));
    Got got = read_into(log, reader);
    CHECK(got.read == blog::Read::item(1) && got.words == entry_of(1), "then the entry published after it opened");
    // A position past the head is clamped to it.
    CHECK(log.reader_at(99).cursor() == 2, "a position past the head is the head");
}

void there_is_one_publisher_and_its_successor_continues_the_sequence() {
    Log log;
    {
        auto first = log.publisher();
        CHECK(first.has_value(), "the first publisher");
        CHECK(!log.publisher().has_value(), "a second publisher is refused");
        CHECK(first->publish(entry_of(0)) == 0, "the first publisher publishes entry 0");
    }
    auto second = log.publisher();
    CHECK(second.has_value(), "the place was freed");
    CHECK(second->publish(entry_of(1)) == 1, "its successor continues the sequence");
}

void publishing_past_the_window_keeps_the_newest_entries() {
    Log log;
    auto publisher = log.publisher();
    for (std::uint64_t n = 0; n < 100; ++n) {
        publisher->publish(entry_of(n));
    }
    blog::Reader reader = log.reader_oldest();
    Got a = read_into(log, reader);
    CHECK(a.read == blog::Read::item(98) && a.words == entry_of(98), "entry 98 is retained");
    Got b = read_into(log, reader);
    CHECK(b.read == blog::Read::item(99) && b.words == entry_of(99), "entry 99 is retained");
    CHECK(read_into(log, reader).read == blog::Read::empty(), "and then the head");
}

// ━━━━━━━━━━━━━━━━━━━━━━ What the C++ row adds to those tests ━━━━━━━━━━━━━━━━━━━━━━

// The loss of one reader is the sum of its lags, not the last one.
void a_readers_loss_is_the_sum_of_its_lags() {
    Log log;
    auto publisher = log.publisher();
    for (std::uint64_t n = 0; n < 5; ++n) {
        publisher->publish(entry_of(n));
    }
    blog::Reader reader = log.reader_at(0);
    CHECK(read_into(log, reader).read == blog::Read::lagged(3), "the first lag skips 3");
    CHECK(reader.cursor() == 3 && reader.lost() == 3, "the reader holds 3 and has lost 3");
    for (std::uint64_t n = 5; n < 9; ++n) {
        publisher->publish(entry_of(n));
    }
    // h = 9: the window is [7, 9), and the reader holds 3.
    CHECK(read_into(log, reader).read == blog::Read::lagged(4), "the second lag skips 4");
    CHECK(reader.cursor() == 7 && reader.lost() == 7, "the loss is the sum, 7");
}

// Two slots retain one entry: each publish overwrites the only other slot.
void a_log_of_two_slots_retains_one_entry() {
    blog::BroadcastLog<3, 2> log;
    auto publisher = log.publisher();
    for (std::uint64_t n = 0; n < 7; ++n) {
        publisher->publish(entry_of(n));
    }
    blog::Reader reader = log.reader_at(0);
    blog::BroadcastLog<3, 2>::Entry out{};
    CHECK(log.read(reader, out) == blog::Read::lagged(6), "of 7 entries one is retained, 6 are behind");
    CHECK(log.read(reader, out) == blog::Read::item(6) && out == entry_of(6), "the retained one is entry 6");
}

// A log that has published its horizon of entries publishes nothing more. A real
// run would take centuries to reach it, so the test holds the predicate `publish`
// asks, at the horizon and on both sides of it, and the stamps it protects.
void a_log_at_its_horizon_publishes_nothing_more() {
    static_assert(blog::kHorizon == (UINT64_MAX - 2) / 2, "the horizon keeps every stamp in a uint64");
    static_assert(blog::detail::done(blog::kHorizon - 1) < UINT64_MAX, "the last stamp fits");
    static_assert(blog::detail::busy(0) == 1 && blog::detail::done(0) == 2, "the stamps of entry 0");
    static_assert(blog::detail::busy(5) == 11 && blog::detail::done(5) == 12, "the stamps of entry 5");
    static_assert(!blog::detail::past_horizon(blog::kHorizon - 1), "the last entry below the horizon is published");
    static_assert(blog::detail::past_horizon(blog::kHorizon), "the horizon itself is refused");
    static_assert(blog::detail::past_horizon(UINT64_MAX), "and so is everything above it");
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher->publish(entry_of(0)) == 0, "a fresh log publishes");
    CHECK(log.head() == 1, "and its head moved");
    CHECK(blog::kNoSequence == UINT64_MAX, "the refusal is not a sequence number");
}

// A log in static storage is constant-initialised and usable before any dynamic
// initialisation has run. (`constinit` makes the compiler reject one that is not.)
constinit blog::BroadcastLog<2, 3> static_log;

void a_log_in_static_storage_is_constant_initialised_and_usable() {
    auto publisher = static_log.publisher();
    CHECK(publisher.has_value(), "the publisher of a static log");
    blog::Reader reader = static_log.reader_oldest();
    CHECK(publisher->publish({41, 42}) == 0, "publish into a static log");
    blog::BroadcastLog<2, 3>::Entry out{};
    CHECK(static_log.read(reader, out) == blog::Read::item(0), "read from a static log");
    CHECK(out[0] == 41 && out[1] == 42, "and the words are the ones published");
}

// A moved publisher keeps the place; the moved-from one gives nothing back.
void a_moved_publisher_keeps_the_place() {
    Log log;
    auto first = log.publisher();
    std::optional<Log::Publisher> moved(std::move(*first));
    first.reset();
    CHECK(!log.publisher().has_value(), "destroying a moved-from publisher frees nothing");
    CHECK(moved->publish(entry_of(0)) == 0, "the moved publisher publishes");
    moved.reset();
    CHECK(log.publisher().has_value(), "destroying the live one frees the place");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Real threads ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
    blog::BroadcastLog<3, Slots> log;
    std::vector<ReaderResult> results(kStressReaders);
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(120);
    std::atomic<bool> publisher_failed{false};

    std::vector<std::thread> threads;
    for (int i = 0; i < kStressReaders; ++i) {
        threads.emplace_back([&, i] {
            ReaderResult &result = results[static_cast<std::size_t>(i)];
            blog::Reader reader = log.reader_at(0);
            std::uint64_t last_item_plus_one = 0;
            typename blog::BroadcastLog<3, Slots>::Entry out{};
            while (reader.cursor() < kStressEntries) {
                const std::uint64_t before = reader.cursor();
                const std::uint64_t lost_before = reader.lost();
                const blog::Read read = log.read(reader, out);
                switch (read.kind) {
                case blog::ReadKind::Item:
                    ++result.items;
                    if (read.seq != before || reader.cursor() != before + 1 || reader.lost() != lost_before ||
                        out != Entry{read.seq + 1, read.seq + 1, read.seq + 1}) {
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
            if (publisher->publish(entry_of(n)) != n) {
                publisher_failed = true;
                return;
            }
            // Now and then give the readers the processor, so that they are
            // sometimes ahead of the publisher and sometimes far behind it.
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
    one_publisher_and_several_readers_see_whole_entries<3>();
    one_publisher_and_several_readers_see_whole_entries<9>();
}

}  // namespace

int main() {
    an_empty_log_is_empty_to_every_reader();
    entries_come_back_in_order_with_their_words();
    the_window_is_exactly_the_retention();
    readers_do_not_share_a_cursor_or_a_loss();
    a_reader_at_the_head_sees_only_what_comes_after();
    there_is_one_publisher_and_its_successor_continues_the_sequence();
    publishing_past_the_window_keeps_the_newest_entries();
    a_readers_loss_is_the_sum_of_its_lags();
    a_log_of_two_slots_retains_one_entry();
    a_log_at_its_horizon_publishes_nothing_more();
    a_log_in_static_storage_is_constant_initialised_and_usable();
    a_moved_publisher_keeps_the_place();
    real_threads_see_whole_entries_and_lose_only_what_they_are_told();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_runtime_test: all checks passed");
    return 0;
}
