// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast-log kind's generated headers for the narrow row (ADR 0062, "Stamp width"),
// compiled and used.
//
// sce-build's tests read what the generator writes for a narrow log and look for text in it,
// which shows the text is what the template says and nothing about whether the names in it are
// the names the narrow runtime and the codecs have, or whether the 32-bit words it packs read
// back. Here the headers the generator writes for two logs are compiled over the element
// headers they include and the runtime's real narrow log (sce/forge/broadcast_log_narrow.h, at
// the 32-bit stamp a row keeps), and the typed functions are used on one thread. The runtime's
// own properties are broadcast_log_narrow_runtime_test.cpp's and
// broadcast_log_narrow_runtime_32_test.cpp's; this file holds what a generated header adds.
//
// The two logs, each generated from a document set by `sce-codegen orchestrate` for the
// machine of a deploy that names the build target and states the longest pause the design
// relies on (the generator chooses the row from the target):
//
//   broadcast_log_cpp_narrow     a target with a 32-bit compare-and-swap (thumbv7em), retention
//                                3 over a record of a tag and a payload of at most eight bytes:
//                                9 bytes, which cross the 4-byte words
//   broadcast_log_cpp_ownership  a target with loads and stores only (thumbv6m), retention 3
//                                over an element of exactly three bytes; its header says to
//                                take the publisher by ownership
//
// The generated headers are held to C++17 by the interface-standard gate (docs/adr/0059,
// section 7); this file is SCE's own source and uses C++20 designated initializers for its
// elements.

#include "broadcast_log_cpp_narrow.h"
#include "broadcast_log_cpp_ownership.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <type_traits>
#include <variant>
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

namespace broadcast = ::SCE::Forge::Broadcast;
namespace narrow = ::SCE::Generated::BroadcastLogCppNarrow;
namespace ownership = ::SCE::Generated::BroadcastLogCppOwnership;

using Record = ::SCE::Generated::BroadcastLogConformanceRecord::BroadcastLogConformanceRecord;
using Event = ::SCE::Generated::BroadcastLogConformanceEvent::BroadcastLogConformanceEvent;

using NarrowLog = narrow::BroadcastLogCppNarrow;
using NarrowItem = narrow::BroadcastLogCppNarrowItem;
using NarrowEmpty = narrow::BroadcastLogCppNarrowEmpty;
using NarrowLagged = narrow::BroadcastLogCppNarrowLagged;
using NarrowRead = narrow::BroadcastLogCppNarrowRead;

using OwnershipLog = ownership::BroadcastLogCppOwnership;
using OwnershipItem = ownership::BroadcastLogCppOwnershipItem;
using OwnershipMalformed = ownership::BroadcastLogCppOwnershipMalformed;
using OwnershipRead = ownership::BroadcastLogCppOwnershipRead;

// What the generated headers state about their entries is checked when this file is compiled:
// a number the template computes wrongly stops the build.
//
// The narrow log: retention 3 is the document's; slots are 3 + 1 = 4, a power of two. The
// record is a one-byte tag and a payload of at most eight bytes: 1 + 8 = 9 bytes, so an entry
// is a length word and ceil(9 / 4) = 3 words of bytes: 1 + 3 = 4 words of 32 bits. The stamp is
// 32 bits, which covers a pause of 2^31 less the 4 slots.
static_assert(narrow::RETENTION == 3, "the retention is the document's: three");
static_assert(narrow::SLOTS == 4, "slots are the retention and one more");
static_assert(narrow::MAX_ENCODED_BYTES == 9, "the record encodes to at most 1 + 8 = 9 bytes");
static_assert(narrow::WORDS == 4, "a length word and three words of bytes: 1 + ceil(9 / 4) = 4");
static_assert(narrow::STAMP_BITS == 32, "the log was judged against a stamp of 32 bits");
static_assert(narrow::STAMP_BITS == broadcast::kNarrowStampBits, "and the runtime keeps it");
static_assert(narrow::WRAP_BOUND_OPS == UINT64_C(2147483644), "2^31 - 4 publishes between two reads of one reader");
static_assert(NarrowLog::kRetention == narrow::RETENTION, "the runtime's retention is the header's");
static_assert(
    std::is_same<NarrowLog, broadcast::BroadcastLog<narrow::WORDS, narrow::SLOTS, broadcast::Narrow<32>>>::value,
    "the log is the one over the narrow lane");
static_assert(std::is_same<NarrowLog::Entry, std::array<std::uint32_t, narrow::WORDS>>::value,
              "an entry is 32-bit words");

// The ownership log: an element of a uint8 and a uint16, 1 + 2 = 3 bytes, so an entry is a
// length word and ceil(3 / 4) = 1 word of bytes: 1 + 1 = 2 words.
static_assert(ownership::RETENTION == 3 && ownership::SLOTS == 4, "retention 3 keeps 4 slots");
static_assert(ownership::MAX_ENCODED_BYTES == 3, "the element encodes to at most 1 + 2 = 3 bytes");
static_assert(ownership::WORDS == 2, "a length word and one word of bytes: 1 + ceil(3 / 4) = 2");
static_assert(ownership::STAMP_BITS == 32, "a stamp of 32 bits");
static_assert(OwnershipLog::kRetention == ownership::RETENTION, "the runtime's retention is the header's");

void the_narrow_header_states_what_the_document_its_element_and_the_target_give() {
    CHECK(narrow::DECLARED_PROGRESS == "wait-free", "the document declared wait-free");
    CHECK(ownership::DECLARED_PROGRESS == "wait-free", "the ownership document declared wait-free");
}

// ─── The narrow log: a record whose bytes cross a 32-bit word ───

Record make_record(std::uint8_t tag, std::size_t length) {
    Record record{};
    record.tag = tag;
    for (std::size_t i = 0; i < length; ++i) {
        record.payload.push_back(static_cast<std::uint8_t>(i + 1));
    }
    return record;
}

std::uint64_t publish_record(narrow::BroadcastLogCppNarrowPublisher &publisher, std::uint8_t tag, std::size_t length) {
    const narrow::BroadcastLogCppNarrowPublished published = narrow::publish(publisher, make_record(tag, length));
    CHECK(published.published() && !published.error.has_value(), "a record within its bound is published");
    return published.seq;
}

// Whether `got` is the entry `seq`, a record of `tag` and `length` payload bytes whole.
bool is_record(const NarrowRead &got, std::uint64_t seq, std::uint8_t tag, std::size_t length) {
    const NarrowItem *item = std::get_if<NarrowItem>(&got);
    if (item == nullptr || item->seq != seq || item->element.tag != tag || item->element.payload.size() != length) {
        return false;
    }
    for (std::size_t i = 0; i < length; ++i) {
        if (item->element.payload[i] != static_cast<std::uint8_t>(i + 1)) {
            return false;
        }
    }
    return true;
}

void records_come_back_whole_and_are_packed_four_bytes_to_a_word() {
    NarrowLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "a new log hands out the publisher's place");
    broadcast::Reader reader = log.reader_oldest();
    CHECK(log.head() == 0, "a new log has published nothing");
    CHECK(std::holds_alternative<NarrowEmpty>(narrow::read(log, reader)), "a new log is empty");

    // The three records encode to 1 + 8 = 9, 1 + 3 = 4 and 1 + 0 = 1 bytes.
    const std::size_t lengths[3] = {8, 3, 0};
    for (std::uint8_t n = 0; n < 3; ++n) {
        CHECK(publish_record(*publisher, static_cast<std::uint8_t>(0xA0 + n), lengths[n]) == n,
              "a record takes the next sequence number");
    }
    CHECK(log.head() == 3, "three publishes: the head is three");

    // The words the runtime holds for entry 0 are the documented ones: the length 9, then the
    // bytes A0 01 02 03 | 04 05 06 07 | 08 little-endian, four to a word.
    {
        broadcast::Reader raw = log.reader_oldest();
        NarrowLog::Entry words{};
        const broadcast::Read found = log.read(raw, words);
        CHECK(found == broadcast::Read::item(0), "the runtime reads entry 0");
        CHECK(words[0] == 9, "the length word is 9");
        CHECK(words[1] == UINT32_C(0x030201A0), "the first word of bytes: A0 01 02 03");
        CHECK(words[2] == UINT32_C(0x07060504), "the second word of bytes: 04 05 06 07");
        CHECK(words[3] == UINT32_C(0x00000008), "the third word holds the ninth byte alone");
    }

    for (std::uint8_t n = 0; n < 3; ++n) {
        CHECK(is_record(narrow::read(log, reader), n, static_cast<std::uint8_t>(0xA0 + n), lengths[n]),
              "a record comes back whole, in order");
    }
    CHECK(std::holds_alternative<NarrowEmpty>(narrow::read(log, reader)), "then the head");
    CHECK(reader.cursor() == 3 && reader.lost() == 0, "the reader holds the next entry and lost none");
}

void a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest() {
    NarrowLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the publisher");
    broadcast::Reader reader = log.reader_oldest();

    // Six entries, 0 to 5, are published before the reader reads. The log retains the last
    // three, so the window is [6 - 3, 6) = [3, 6), and the reader at 0 is outside it: it has
    // missed (6 - 3) - 0 = 3 entries.
    for (std::uint8_t n = 0; n < 6; ++n) {
        (void)publish_record(*publisher, static_cast<std::uint8_t>(0x10 + n), 0);
    }
    const NarrowRead lapped = narrow::read(log, reader);
    const NarrowLagged *lag = std::get_if<NarrowLagged>(&lapped);
    CHECK(lag != nullptr && lag->skipped == 3, "the reader is told it missed three");
    CHECK(reader.cursor() == 3 && reader.lost() == 3, "it resumes at the oldest retained and carries the loss");
    for (std::uint8_t n = 3; n < 6; ++n) {
        CHECK(is_record(narrow::read(log, reader), n, static_cast<std::uint8_t>(0x10 + n), 0),
              "what is held is read whole");
    }
    CHECK(std::holds_alternative<NarrowEmpty>(narrow::read(log, reader)), "then nothing is new");
    CHECK(reader.lost() == 3, "reading what is retained loses nothing more");
}

// A deferred reader binds at its first read, to the oldest entry retained then, and is told of
// nothing; the reader opened at the oldest before the publishes is told it missed some.
void a_deferred_reader_binds_at_its_first_typed_read() {
    NarrowLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the publisher");
    broadcast::Reader eager = log.reader_oldest();
    broadcast::Reader deferred = log.reader_deferred();
    CHECK(!deferred.is_bound(), "a deferred reader has no cursor before its first read");
    for (std::uint8_t n = 0; n < 6; ++n) {
        (void)publish_record(*publisher, static_cast<std::uint8_t>(0x20 + n), 1);
    }
    CHECK(is_record(narrow::read(log, deferred), 3, 0x23, 1), "the first read is the oldest retained entry, 3");
    CHECK(deferred.is_bound() && deferred.lost() == 0, "the first read bound it, and it was told of nothing");
    const NarrowRead lapped = narrow::read(log, eager);
    const NarrowLagged *lag = std::get_if<NarrowLagged>(&lapped);
    CHECK(lag != nullptr && lag->skipped == 3, "the reader opened before was told it missed three");
}

// The head, a reader's cursor and a sequence number are residues modulo 2^32, and a reader
// opened at one the log has not reached is held at the head.
void a_position_is_a_residue_and_one_past_the_head_is_the_head() {
    NarrowLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the publisher");
    (void)publish_record(*publisher, 1, 0);
    (void)publish_record(*publisher, 2, 0);
    CHECK(log.reader_at(1).cursor() == 1, "a reader asked for entry 1, which is 1 behind the head, holds it");
    CHECK(log.reader_at(99).cursor() == 2, "a reader asked for entry 99 of a log of two holds the head");
    CHECK(log.reader_head().cursor() == 2, "a reader at the head holds it");
}

void the_publishers_place_is_one_whichever_form_takes_it() {
    NarrowLog log;
    {
        auto first = log.publisher();
        CHECK(first.has_value(), "the publisher's place is free");
        CHECK(!log.publisher().has_value(), "a log written by two publishers is not this algorithm");
        CHECK(!log.split().has_value(), "and the ownership form refuses the place the exchange holds");
        CHECK(publish_record(*first, 1, 0) == 0 && publish_record(*first, 2, 0) == 1, "two entries: 0 and 1");
    }
    {
        auto second = log.split();
        CHECK(second.has_value(), "the ownership form takes the place once it is given back");
        CHECK(!log.publisher().has_value(), "which the exchange then finds taken");
        CHECK(publish_record(*second, 3, 0) == 2, "the successor continues the sequence where the last stopped");
    }
    {
        NarrowLog::Publisher unchecked = log.publisher_unchecked();
        CHECK(publish_record(unchecked, 4, 0) == 3, "the publisher taken on the caller's word continues it too");
        CHECK(!log.split().has_value(), "and holds the place against the others");
    }
}

// ─── The ownership log: a target with loads and stores only ───

Event make_event(std::uint8_t sensor, std::uint16_t value) {
    return Event{sensor, value};
}

std::uint64_t publish_event(OwnershipLog::Publisher &publisher, std::uint8_t sensor, std::uint16_t value) {
    const ownership::BroadcastLogCppOwnershipPublished published =
        ownership::publish(publisher, make_event(sensor, value));
    CHECK(published.published() && !published.error.has_value(), "an element within its bound is published");
    return published.seq;
}

bool is_event(const OwnershipRead &got, std::uint64_t seq, std::uint8_t sensor, std::uint16_t value) {
    const OwnershipItem *item = std::get_if<OwnershipItem>(&got);
    return item != nullptr && item->seq == seq && item->element.sensor_id() == sensor && item->element.value() == value;
}

void entries_come_back_through_the_ownership_forms() {
    static OwnershipLog log;
    auto publisher = log.split();
    CHECK(publisher.has_value(), "a log the caller holds exclusively hands out its place");
    broadcast::Reader reader = log.reader_oldest();
    for (std::uint8_t i = 0; i < 3; ++i) {
        CHECK(publish_event(*publisher, static_cast<std::uint8_t>(10 + i), static_cast<std::uint16_t>(1000 + i)) == i,
              "an entry takes the next sequence number");
    }
    for (std::uint8_t i = 0; i < 3; ++i) {
        CHECK(is_event(ownership::read(log, reader), i, static_cast<std::uint8_t>(10 + i),
                       static_cast<std::uint16_t>(1000 + i)),
              "an entry comes back whole");
    }
    publisher.reset();

    // A log in static storage is shared from the start: its successor is taken on the caller's
    // word and continues the sequence.
    OwnershipLog::Publisher successor = log.publisher_unchecked();
    CHECK(publish_event(successor, 99, 9999) == 3, "the unchecked publisher continues at 3");
    CHECK(is_event(ownership::read(log, reader), 3, 99, 9999), "and its entry is read");
    CHECK(log.head() == 4, "four entries are published");
}

// What the log reads back is only what the typed publish wrote; the runtime's own publish
// writes any words, and a read refuses what no publish of this log wrote.
void a_stored_length_above_the_bound_is_refused_and_never_copied() {
    OwnershipLog log;
    OwnershipLog::Publisher publisher = log.publisher_unchecked();
    broadcast::Reader reader = log.reader_head();

    // A length of 4 is one past the bound of 3; the largest length a word holds is not an
    // index either.
    for (const std::uint32_t stored : {std::uint32_t{4}, UINT32_MAX}) {
        OwnershipLog::Entry entry{};
        entry[0] = stored;
        const std::uint64_t seq = publisher.publish(entry);
        const OwnershipRead got = ownership::read(log, reader);
        const OwnershipMalformed *bad = std::get_if<OwnershipMalformed>(&got);
        CHECK(bad != nullptr && bad->seq == seq, "a length above the bound is refused by its entry");
    }

    // A length of exactly the bound, three bytes 0x0A 0xE8 0x03, is the element sensor 10 and
    // value 0x03E8 = 1000, little-endian in the word: 0x0A + 0xE8 * 2^8 + 0x03 * 2^16 = 256010.
    OwnershipLog::Entry exact{};
    exact[0] = 3;
    exact[1] = 256010;
    const std::uint64_t exact_seq = publisher.publish(exact);
    CHECK(is_event(ownership::read(log, reader), exact_seq, 10, 1000), "a length of exactly the bound is read");

    // One byte stored is within the bound and is not an element: the codec says so.
    OwnershipLog::Entry short_entry{};
    short_entry[0] = 1;
    short_entry[1] = 0x0A;
    const std::uint64_t short_seq = publisher.publish(short_entry);
    const OwnershipRead got = ownership::read(log, reader);
    const OwnershipMalformed *bad = std::get_if<OwnershipMalformed>(&got);
    CHECK(bad != nullptr && bad->seq == short_seq, "a frame the codec refuses is not an item");
}

}  // namespace

int main() {
    the_narrow_header_states_what_the_document_its_element_and_the_target_give();
    records_come_back_whole_and_are_packed_four_bytes_to_a_word();
    a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest();
    a_deferred_reader_binds_at_its_first_typed_read();
    a_position_is_a_residue_and_one_past_the_head_is_the_head();
    the_publishers_place_is_one_whichever_form_takes_it();
    entries_come_back_through_the_ownership_forms();
    a_stored_length_above_the_bound_is_refused_and_never_copied();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_narrow_generated_test: all checks passed");
    return 0;
}
