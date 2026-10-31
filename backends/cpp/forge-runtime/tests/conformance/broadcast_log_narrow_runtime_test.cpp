// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The C++ arm of the broadcast-log kind's narrow row (ADR 0062, "Stamp width"): the unit
// tests of the Rust lane `Narrow<3>` (backends/rust/forge-runtime/src/broadcast_log_core.rs)
// and of the C11 row (broadcast_log_narrow_runtime_test.c) run against
// sce/forge/broadcast_log_narrow.h, over a stamp of 3 bits.
//
// A stamp of 3 bits gives a stamp space of M = 8 values, half of it K = 4, so the head, the
// stamps and the cursors wrap after eight publishes: the cases that a stamp of 32 bits could
// not reach in a run of any length are a few publishes away. The log has 2 slots (a retention
// of 1) unless a test says otherwise, and an entry is one word holding its true count and not
// its residue, so a copy that is the entry of the wrong lap shows as a word of another count.
//
// The 32-bit row, the ownership forms and real threads are
// broadcast_log_narrow_runtime_32_test.cpp's, which keeps the header's own width.

#include "sce/forge/broadcast_log_narrow.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <type_traits>

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

using Lane = blog::Narrow<3>;
using Arith = blog::detail::StampArith<3>;
// Two slots retain one entry; four slots, as many as K, retain three.
using Log2 = blog::BroadcastLog<1, 2, Lane>;
using Log4 = blog::BroadcastLog<1, 4, Lane>;

template <class LogT> typename LogT::Entry entry_of(std::uint32_t count) {
    return typename LogT::Entry{count};
}

template <class LogT> struct Got {
    blog::Read read;
    typename LogT::Entry words;
};

// One read into a buffer of the sentinel, so a read that wrote nothing is seen.
template <class LogT> Got<LogT> read_into(const LogT &log, blog::Reader &reader) {
    Got<LogT> got{blog::Read::empty(), typename LogT::Entry{UINT32_MAX}};
    got.read = log.read(reader, got.words);
    return got;
}

// Publishes the entries `from` to `to - 1`: the word of an entry is its true count.
template <class LogT> void publish_all(typename LogT::Publisher &publisher, std::uint32_t from, std::uint32_t to) {
    for (std::uint32_t count = from; count < to; ++count) {
        (void)publisher.publish(entry_of<LogT>(count));
    }
}

// ─── The width gives the stamp space ───

void the_stamp_space_and_its_half_follow_from_the_width() {
    static_assert(Arith::kMask == 7, "2^3 - 1");
    static_assert(Arith::kSpace == 8, "2^3");
    static_assert(Arith::kHalf == 4, "2^(3 - 1)");
    static_assert(Lane::kBits == 3 && Lane::kUnwritten == 1, "the lane keeps the width it was named with");
    static_assert(Log2::kRetention == 1, "2 slots keep 1");
    static_assert(Log4::kRetention == 3, "4 slots keep 3");
    static_assert(std::is_same<Log2::Entry, std::array<std::uint32_t, 1>>::value,
                  "the words of a narrow entry are 32-bit");
    CHECK(Arith::kMask == 7, "the mask is the low three bits");
}

// done(n) = 2n + 2 mod 8, so n and n + 4 give the same stamp (2n + 10 = 2n + 2 mod 8) and
// n and n + j for j = 1, 2, 3 differ by 2j = 2, 4, 6.
void a_stamp_tells_apart_exactly_half_the_stamp_space() {
    for (std::uint64_t n = 0; n < 8; ++n) {
        CHECK(Arith::done(n) == Arith::done(n + 4), "done one half-space on is the same stamp");
        CHECK(Arith::busy(n) == Arith::busy(n + 4), "busy one half-space on is the same stamp");
        for (std::uint64_t j = 1; j < 4; ++j) {
            CHECK(Arith::done(n) != Arith::done(n + j), "done within a half-space is another stamp");
            CHECK(Arith::busy(n) != Arith::busy(n + j), "busy within a half-space is another stamp");
        }
        CHECK(Arith::busy(n) != Arith::done(n), "busy is odd and done is even");
    }
}

// The stamps of the last slot, over four laps of the stamp space (the count reduced as a
// publisher reduces it), are never the value that stands for an unwritten slot.
void the_last_slot_is_never_stamped_unwritten(std::uint64_t slots) {
    for (std::uint64_t count = 0; count < 32; ++count) {
        const std::uint64_t n = count & Arith::kMask;
        if (n % slots == slots - 1) {
            CHECK(Arith::busy(n) != Lane::kUnwritten, "no busy stamp of the last slot is the unwritten one");
            CHECK(Arith::done(n) != Lane::kUnwritten, "no done stamp of the last slot is the unwritten one");
        }
    }
}

void the_unwritten_stamp_is_never_one_the_protocol_writes_into_the_last_slot() {
    the_last_slot_is_never_stamped_unwritten(2);
    the_last_slot_is_never_stamped_unwritten(4);
}

void a_stamp_at_the_top_of_the_space_is_what_the_arithmetic_says() {
    CHECK(Arith::done(7) == 0, "2 * 7 + 2 = 16, 0 modulo 8");
    CHECK(Arith::busy(7) == 7, "2 * 7 + 1 = 15, 7 modulo 8");
}

// ─── The wrap ───

// Twelve entries cross the head's wrap at 8 once. The reader keeps up, so after publish `n`
// it finds entry `n` and no other: its sequence number is `n mod 8`, and the stored head is
// `(n + 1) mod 8`.
void a_narrow_head_wraps_and_the_reader_still_reads_every_entry_in_order() {
    Log2 log;
    auto publisher = log.split();
    CHECK(publisher.has_value(), "the publisher's place is free in a new log");
    blog::Reader reader = log.reader_head();
    CHECK(reader.cursor() == 0, "a reader at the head of a new log holds 0");
    for (std::uint32_t n = 0; n < 12; ++n) {
        CHECK(publisher->publish(entry_of<Log2>(n)) == n % 8, "publish takes the sequence number modulo 8");
        CHECK(log.head() == (n + 1) % 8, "the head is the count modulo 8");
        const Got<Log2> item = read_into(log, reader);
        CHECK(item.read == blog::Read::item(n % 8) && item.words == entry_of<Log2>(n), "the entry just published");
        CHECK(read_into(log, reader).read == blog::Read::empty(), "then the head");
    }
    CHECK(reader.lost() == 0, "a reader that kept up lost nothing");
}

// Six entries (0 to 5): the reader opened now is at 6. Three more (6, 7, 8): the head is 9,
// stored as 9 mod 8 = 1. The reader is (1 - 6) mod 8 = 3 behind, which is the window (1
// entry) and 2 more.
void a_reader_lapped_across_the_wrap_is_told_the_exact_count() {
    Log2 log;
    auto publisher = log.split();
    publish_all<Log2>(*publisher, 0, 6);
    blog::Reader reader = log.reader_head();
    CHECK(reader.cursor() == 6, "the reader holds 6");
    publish_all<Log2>(*publisher, 6, 9);
    CHECK(log.head() == 1, "9 publishes leave the head at 1 modulo 8");
    CHECK(read_into(log, reader).read == blog::Read::lagged(2), "the reader is told it missed 2");
    // 6 + 2 = 8, which is 0 modulo 8: the oldest retained entry is 8.
    CHECK(reader.cursor() == 0, "the cursor is the residue of the oldest retained entry");
    CHECK(reader.lost() == 2, "and the loss is carried");
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(8),
          "the entry of count 8, whose stamp is the one of count 0 a lap before");
    CHECK(read_into(log, reader).read == blog::Read::empty(), "and then the head");
}

// Outside the bound the answer is defined and is what the arithmetic of the cursor says;
// these four hold each outcome.

// Eight entries: the head is 8 mod 8 = 0, where the reader is. It cannot tell that from
// nothing published, which is outside the bound, and it is told what the arithmetic says.
void a_reader_one_stamp_space_behind_is_told_empty_and_keeps_its_place() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_head();
    publish_all<Log2>(*publisher, 0, 8);
    CHECK(read_into(log, reader).read == blog::Read::empty(), "a reader a stamp space behind is told empty");
    CHECK(reader.cursor() == 0, "its cursor stays");
    CHECK(reader.lost() == 0, "the 8 entries it missed are not counted");
    // Its place is right: the next entry, count 8, is the one it reads.
    publish_all<Log2>(*publisher, 8, 9);
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(8), "and it reads entry 8 next");
}

// Ten entries (0 to 9): the head is 10 mod 8 = 2, so the reader is 2 behind, which is the
// window (1) and 1 more: skipped is 1, where the truth is 10 - 1 = 9, short by 8.
void a_reader_more_than_a_stamp_space_behind_is_lagged_by_the_count_modulo_the_space() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_head();
    publish_all<Log2>(*publisher, 0, 10);
    CHECK(read_into(log, reader).read == blog::Read::lagged(1), "skipped is the distance modulo 8 less the retention");
    CHECK(reader.lost() == 1, "and that is the loss");
    // The place is right all the same: 0 + 1 = 1 is the residue of count 9, the oldest
    // retained, and it is read whole.
    CHECK(reader.cursor() == 1, "the cursor is the oldest retained entry's residue");
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(1) && item.words == entry_of<Log2>(9), "entry 9 is read whole");
}

// Nine entries (0 to 8): the head is 9 mod 8 = 1, one ahead of the reader, inside the
// window; the slot of entry 0 holds entry 8, whose stamp done(8 mod 8) = 2 is done(0): the
// copy validates and is handed as the entry the reader wanted. This is what the bound exists
// to keep from happening, and why the deploy states it.
void a_reader_a_stamp_space_and_one_behind_is_handed_a_later_lap_as_its_own() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_head();
    publish_all<Log2>(*publisher, 0, 9);
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(8),
          "the entry of a lap on is handed as entry 0");
}

// A mismatch proves the publisher is `Slots` or more ahead. A head that says the reader is
// only 1 behind is a head one lap round: 1 + 8 = 9 behind, less the retention 1, is 8
// skipped, and the cursor (5 + 8) mod 8 = 5 is where it was. The reader cannot be put into
// that state from outside a read, so the function every lag is counted by is held directly.
void a_head_that_wrapped_during_a_mismatch_still_gives_a_lag_of_at_least_one() {
    CHECK(Arith::missed(1, 2, 1) == 8, "a lag is never reported as skipping nothing");
    CHECK(Arith::advance(5, Arith::missed(1, 2, 1)) == 5, "the cursor is where it was");
    // A head that already says `Slots` or more is taken as it is: 3 - 1 = 2.
    CHECK(Arith::missed(3, 2, 1) == 2, "a distance of 3 less the retention 1");
    CHECK(Arith::advance(5, Arith::missed(3, 2, 1)) == 7, "(5 + 2) mod 8");
    // A distance of the slot count is a lag of 1, and one below it has a lap added.
    CHECK(Arith::missed(2, 2, 1) == 1, "the slot count itself is a lag of 1");
    CHECK(Arith::missed(0, 4, 3) == 5, "a lap (8) less the retention (3), from a distance of 0");
}

// ─── Opening a reader ───

void the_oldest_reader_knows_a_young_log_from_one_that_wrapped() {
    Log2 log;
    auto publisher = log.split();
    // Nothing published: the oldest retained entry is 0 and the reader is at 0.
    blog::Reader reader = log.reader_oldest();
    CHECK(reader.cursor() == 0 && read_into(log, reader).read == blog::Read::empty(),
          "an empty log's oldest reader is at the head");
    // One entry: still `h <= retention = 1`, so the oldest is 0.
    publish_all<Log2>(*publisher, 0, 1);
    reader = log.reader_oldest();
    Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(0), "after one entry the oldest is entry 0");
    // Two entries: slot 1 is written, so the oldest retained is 2 - 1 = 1.
    publish_all<Log2>(*publisher, 1, 2);
    reader = log.reader_oldest();
    CHECK(reader.cursor() == 1, "after two entries the oldest is entry 1");
    item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(1) && item.words == entry_of<Log2>(1), "and it is read");
    // After the wrap, with the head at a small residue: 8 entries leave the head at 0, and
    // the oldest retained is count 7, residue (0 - 1) mod 8 = 7.
    publish_all<Log2>(*publisher, 2, 8);
    reader = log.reader_oldest();
    CHECK(reader.cursor() == 7, "with the head at 0 the oldest is residue 7");
    item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(7) && item.words == entry_of<Log2>(7), "entry 7");
    // Nine: the head is 1, the oldest is count 8, residue 0.
    publish_all<Log2>(*publisher, 8, 9);
    reader = log.reader_oldest();
    CHECK(reader.cursor() == 0, "with the head at 1 the oldest is residue 0");
    item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(8), "entry 8, by its residue");
}

// ─── The deferred reader ───

// Both readers are opened on the empty log, one at the oldest now and one deferred; then
// three entries (0, 1, 2) are published. Retention is 1, so the oldest retained is entry 2.
void a_deferred_reader_starts_at_the_oldest_of_its_first_read_and_is_told_of_nothing() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader eager = log.reader_oldest();
    blog::Reader deferred = log.reader_deferred();
    CHECK(eager.is_bound(), "a reader opened at the oldest is bound");
    CHECK(!deferred.is_bound(), "a deferred reader is not bound before its first read");
    CHECK(deferred.cursor() == 0 && deferred.lost() == 0, "no cursor yet reads as 0, with no loss");
    publish_all<Log2>(*publisher, 0, 3);
    // The eager reader holds 0, which is 3 behind: it is told it missed 2.
    CHECK(read_into(log, eager).read == blog::Read::lagged(2), "the eager reader is told of the lag");
    // The deferred reader binds at this read, at the oldest retained entry.
    const Got<Log2> first = read_into(log, deferred);
    CHECK(first.read == blog::Read::item(2) && first.words == entry_of<Log2>(2),
          "the first read of a deferred reader is the oldest retained entry");
    CHECK(deferred.is_bound(), "the first read binds it");
    CHECK(deferred.lost() == 0, "a reader that waited is told of nothing");
    CHECK(deferred.cursor() == 3, "and holds the entry after the one it read");
    CHECK(read_into(log, deferred).read == blog::Read::empty(), "then the head");
}

void the_other_constructors_give_bound_readers() {
    Log2 log;
    CHECK(log.reader_oldest().is_bound(), "oldest is bound");
    CHECK(log.reader_head().is_bound(), "head is bound");
    CHECK(log.reader_at(0).is_bound(), "at is bound");
}

void a_deferred_reader_on_a_log_with_nothing_published_is_empty_and_then_reads_from_the_start() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_deferred();
    CHECK(read_into(log, reader).read == blog::Read::empty(), "nothing published is empty");
    CHECK(reader.is_bound(), "the empty read bound it");
    CHECK(reader.cursor() == 0, "at the start of the log");
    publish_all<Log2>(*publisher, 0, 1);
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log2>(0),
          "the first entry is read from the start");
    CHECK(reader.lost() == 0, "with nothing lost");
}

// Ten entries cross the head's wrap at 8, so the head is 2 and the last slot is written. The
// oldest retained is entry 9, cursor 9 mod 8 = 1.
void a_deferred_reader_across_the_wrap_binds_to_the_oldest_by_the_last_slot() {
    Log2 log;
    auto publisher = log.split();
    publish_all<Log2>(*publisher, 0, 10);
    CHECK(log.head() == 2, "ten publishes leave the head at 2 modulo 8");
    blog::Reader reader = log.reader_deferred();
    const Got<Log2> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(1) && item.words == entry_of<Log2>(9), "the oldest retained is entry 9");
    CHECK(reader.lost() == 0, "and nothing is lost");
    // A reader that falls behind later is told as any reader is: three more entries leave
    // the head at 5, the reader holding 2 is 3 behind, and misses 3 - 1 = 2.
    publish_all<Log2>(*publisher, 10, 13);
    CHECK(read_into(log, reader).read == blog::Read::lagged(2), "a later lag is told");
    CHECK(reader.lost() == 2 && reader.cursor() == 4, "and counted, at the residue of entry 12");
}

// Three entries of a log that retains three: its last slot was never written, so the oldest
// is entry 0 though the head is 3.
void a_deferred_reader_on_a_young_log_binds_to_entry_zero_by_the_last_slot() {
    Log4 log;
    auto publisher = log.split();
    publish_all<Log4>(*publisher, 0, 3);
    blog::Reader reader = log.reader_deferred();
    const Got<Log4> item = read_into(log, reader);
    CHECK(item.read == blog::Read::item(0) && item.words == entry_of<Log4>(0), "the oldest is entry 0");
    CHECK(reader.lost() == 0, "and nothing is lost");
}

// The bind and `reader_oldest` give one cursor, whatever the log has published and whichever
// side of the wrap the head is.
template <class LogT> void a_deferred_reader_binds_where_the_oldest_reader_opens_in() {
    LogT log;
    auto publisher = log.split();
    for (std::uint32_t count = 0; count < 20; ++count) {
        blog::Reader deferred = log.reader_deferred();
        const blog::Reader oldest = log.reader_oldest();
        const Got<LogT> got = read_into(log, deferred);
        if (count == 0) {
            CHECK(got.read == blog::Read::empty(), "an empty log is empty to a deferred reader");
        } else {
            CHECK(got.read == blog::Read::item(oldest.cursor()),
                  "a deferred reader reads what the oldest reader opens at");
        }
        CHECK(deferred.lost() == 0 && deferred.is_bound(), "bound, none lost");
        publish_all<LogT>(*publisher, count, count + 1);
    }
}

void a_deferred_reader_binds_where_the_oldest_reader_opens() {
    a_deferred_reader_binds_where_the_oldest_reader_opens_in<Log2>();
    a_deferred_reader_binds_where_the_oldest_reader_opens_in<Log4>();
}

// The head is 6; K = 4. A position fewer than 4 behind is taken as it is.
void a_position_is_read_modulo_the_stamp_space_and_one_ahead_is_the_head() {
    Log2 log;
    auto publisher = log.split();
    publish_all<Log2>(*publisher, 0, 6);
    CHECK(log.reader_at(3).cursor() == 3, "(6 - 3) mod 8 = 3 < 4");
    CHECK(log.reader_at(6).cursor() == 6, "at the head");
    // 4 or more behind is what one ahead of the head looks like, and is the head:
    // (6 - 2) mod 8 = 4, and (6 - 7) mod 8 = 7.
    CHECK(log.reader_at(2).cursor() == 6, "4 behind is the head");
    CHECK(log.reader_at(7).cursor() == 6, "one ahead is the head");
    // A count past the stamp space is reduced: 14 mod 8 = 6.
    CHECK(log.reader_at(14).cursor() == 6, "a count past the stamp space is reduced");
    CHECK(log.reader_at(UINT64_C(0x100000006)).cursor() == 6, "a wide count is reduced as well");
}

// ─── The slot count ───

// 3 retained of 4 slots over 8 stamp values: ten entries, read one at a time, then a reader
// that was held back for 3.
void a_log_with_as_many_slots_as_half_the_stamp_space_keeps_its_window_across_the_wrap() {
    Log4 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_head();
    for (std::uint32_t n = 0; n < 10; ++n) {
        publish_all<Log4>(*publisher, n, n + 1);
        const Got<Log4> item = read_into(log, reader);
        CHECK(item.read == blog::Read::item(n % 8) && item.words == entry_of<Log4>(n),
              "entry by entry across the wrap");
    }
    publish_all<Log4>(*publisher, 10, 13);
    // A reader that waited through 3 publishes is 3 behind the head, inside the window of 3,
    // and reads the oldest retained entry, count 10, first.
    blog::Reader held = log.reader_head();
    CHECK(held.cursor() == 13 % 8, "the head is 13 modulo 8");
    blog::Reader behind = log.reader_at(10);
    CHECK(behind.cursor() == 10 % 8, "(5 - 2) mod 8 = 3 behind: in the window");
    const Got<Log4> item = read_into(log, behind);
    CHECK(item.read == blog::Read::item(2) && item.words == entry_of<Log4>(10), "the oldest retained entry first");
    CHECK(read_into(log, held).read == blog::Read::empty(), "the reader at the head sees nothing");
}

// The slot count must divide the stamp space: a power of two, at most K. A log of another
// shape is a compile error where it is built (the class asserts exactly this predicate), so
// the predicate is what is held.
void the_class_refuses_a_slot_count_the_stamp_cannot_number() {
    static_assert(!Arith::slots_fit(3), "3 slots do not divide 8");
    static_assert(!Arith::slots_fit(6), "6 slots do not divide 8");
    static_assert(!Arith::slots_fit(8), "8 slots are more than half the stamp space");
    static_assert(!Arith::slots_fit(1), "one slot would retain nothing");
    static_assert(!Arith::slots_fit(0), "no slot would retain nothing");
    static_assert(Arith::slots_fit(2), "two slots retain one entry");
    static_assert(Arith::slots_fit(4), "four slots are half the stamp space and are kept");
    static_assert(blog::detail::StampArith<32>::slots_fit(UINT64_C(0x80000000)), "half of 2^32 is the most at 32 bits");
    static_assert(!blog::detail::StampArith<32>::slots_fit(UINT64_C(0xC0000000)), "not a power of two at 32 bits");
    CHECK(Arith::slots_fit(2) && !Arith::slots_fit(3), "the predicate is the one the class asserts");
}

// An entry of three words comes back whole across the wrap, each word of its own.
void an_entry_of_several_words_comes_back_whole() {
    blog::BroadcastLog<3, 2, Lane> log;
    auto publisher = log.split();
    CHECK(publisher.has_value(), "the publisher");
    blog::Reader reader = log.reader_head();
    for (std::uint32_t n = 0; n < 10; ++n) {
        const blog::BroadcastLog<3, 2, Lane>::Entry entry{n + 1, n + 100, UINT32_C(0xDEADBEEF) - n};
        (void)publisher->publish(entry);
        blog::BroadcastLog<3, 2, Lane>::Entry out{0, 0, 0};
        CHECK(log.read(reader, out) == blog::Read::item(n % 8), "an entry is an item");
        CHECK(out == entry, "and is whole");
    }
}

// ─── The publisher ───

void a_split_publisher_holds_the_place_and_its_successor_continues_the_sequence() {
    Log2 log;
    {
        auto first = log.split();
        CHECK(first.has_value(), "the place is free");
        CHECK(!log.split().has_value(), "a second place is refused by the ownership form");
        CHECK(!log.publisher().has_value(), "and by the exchange");
        CHECK(first->publish(entry_of<Log2>(0)) == 0, "the first publisher publishes entry 0");
        CHECK(first->publish(entry_of<Log2>(1)) == 1, "and entry 1");
    }
    // The place is free again, and the successor continues where the head says.
    {
        auto second = log.publisher();
        CHECK(second.has_value(), "the exchange takes the freed place");
        CHECK(second->publish(entry_of<Log2>(2)) == 2, "the sequence continues");
        CHECK(!log.split().has_value(), "a place taken by the exchange refuses the ownership form");
    }
    // A log in static storage takes it on the caller's word, and the flag is raised.
    {
        Log2::Publisher unchecked = log.publisher_unchecked();
        CHECK(unchecked.publish(entry_of<Log2>(3)) == 3, "the unchecked publisher continues the sequence");
        CHECK(!log.publisher().has_value(), "and holds the place against the exchange");
        CHECK(!log.split().has_value(), "and against the ownership form");
    }
    CHECK(log.publisher().has_value(), "released, the place is free");
}

// A narrow count has no horizon: 100 entries are 12 laps of the stamp space and 4 entries, and
// the reader that keeps up reads each by its residue.
void publishing_forever_does_not_reach_a_horizon_in_a_narrow_log() {
    Log2 log;
    auto publisher = log.split();
    blog::Reader reader = log.reader_head();
    for (std::uint32_t n = 0; n < 100; ++n) {
        publish_all<Log2>(*publisher, n, n + 1);
        const Got<Log2> item = read_into(log, reader);
        CHECK(item.read == blog::Read::item(n % 8) && item.words == entry_of<Log2>(n), "entry by its residue");
    }
    CHECK(reader.lost() == 0, "nothing was lost");
}

}  // namespace

int main() {
    the_stamp_space_and_its_half_follow_from_the_width();
    a_stamp_tells_apart_exactly_half_the_stamp_space();
    the_unwritten_stamp_is_never_one_the_protocol_writes_into_the_last_slot();
    a_stamp_at_the_top_of_the_space_is_what_the_arithmetic_says();
    a_narrow_head_wraps_and_the_reader_still_reads_every_entry_in_order();
    a_reader_lapped_across_the_wrap_is_told_the_exact_count();
    a_reader_one_stamp_space_behind_is_told_empty_and_keeps_its_place();
    a_reader_more_than_a_stamp_space_behind_is_lagged_by_the_count_modulo_the_space();
    a_reader_a_stamp_space_and_one_behind_is_handed_a_later_lap_as_its_own();
    a_head_that_wrapped_during_a_mismatch_still_gives_a_lag_of_at_least_one();
    the_oldest_reader_knows_a_young_log_from_one_that_wrapped();
    a_deferred_reader_starts_at_the_oldest_of_its_first_read_and_is_told_of_nothing();
    the_other_constructors_give_bound_readers();
    a_deferred_reader_on_a_log_with_nothing_published_is_empty_and_then_reads_from_the_start();
    a_deferred_reader_across_the_wrap_binds_to_the_oldest_by_the_last_slot();
    a_deferred_reader_on_a_young_log_binds_to_entry_zero_by_the_last_slot();
    a_deferred_reader_binds_where_the_oldest_reader_opens();
    a_position_is_read_modulo_the_stamp_space_and_one_ahead_is_the_head();
    a_log_with_as_many_slots_as_half_the_stamp_space_keeps_its_window_across_the_wrap();
    the_class_refuses_a_slot_count_the_stamp_cannot_number();
    an_entry_of_several_words_comes_back_whole();
    a_split_publisher_holds_the_place_and_its_successor_continues_the_sequence();
    publishing_forever_does_not_reach_a_horizon_in_a_narrow_log();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_narrow_runtime_test: all checks passed");
    return 0;
}
