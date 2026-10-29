// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast-log kind's generated header (RFC §synth-5-S, ADR 0062), compiled and run.
//
// sce-build's tests read what the generator writes for a log and look for text in
// it, which shows the text is what the template says and nothing about whether the
// names in it are the names sce/forge/broadcast_log.h has, or whether the bytes it
// packs into an entry read back as the element that was published. Here the
// headers the generator writes for two logs are compiled into this test over the
// element codecs they name, and the typed `publish` and `read` they define are
// used, on one thread: a rename in the runtime, a width the template computes
// wrongly, or a byte packed into the wrong word stops the build or fails a check.
// What a log does under several threads is broadcast_log_runtime_test's and the
// history recorder's, and is not repeated here.
//
// The headers come from tests/forge/resources/broadcast_log_conformance_*.scxml,
// generated at build time as one document set (`sce-codegen orchestrate`) and not
// document by document: an entry is sized from the element document, and the
// second log's retention is a deploy key, resolved against the deploy this
// harness's CMakeLists.txt writes. The generated headers are held to C++17 by the
// interface-standard gate (docs/adr/0059, section 7); this file is SCE's own
// source and uses C++20 designated initializers for its elements.

#include "broadcast_log_conformance_deployed.h"
#include "broadcast_log_conformance_event.h"
#include "broadcast_log_conformance_literal.h"
#include "broadcast_log_conformance_record.h"

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
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
namespace literal = ::SCE::Generated::BroadcastLogConformanceLiteral;
namespace deployed = ::SCE::Generated::BroadcastLogConformanceDeployed;

using Event = ::SCE::Generated::BroadcastLogConformanceEvent::BroadcastLogConformanceEvent;
using Record = ::SCE::Generated::BroadcastLogConformanceRecord::BroadcastLogConformanceRecord;

using Log = literal::BroadcastLogConformanceLiteral;
using Item = literal::BroadcastLogConformanceLiteralItem;
using Empty = literal::BroadcastLogConformanceLiteralEmpty;
using Lagged = literal::BroadcastLogConformanceLiteralLagged;
using Malformed = literal::BroadcastLogConformanceLiteralMalformed;
using Read = literal::BroadcastLogConformanceLiteralRead;

using DeployedLog = deployed::BroadcastLogConformanceDeployed;
using DeployedItem = deployed::BroadcastLogConformanceDeployedItem;
using DeployedEmpty = deployed::BroadcastLogConformanceDeployedEmpty;
using DeployedRead = deployed::BroadcastLogConformanceDeployedRead;

Event event(std::uint8_t sensor_id, std::uint16_t value) {
    return Event{.sensor_id = sensor_id, .value = value};
}

/// Whether `got` is the entry `seq`, holding the event (`sensor_id`, `value`).
bool is_item(const Read &got, std::uint64_t seq, std::uint8_t sensor_id, std::uint16_t value) {
    const Item *item = std::get_if<Item>(&got);
    return item != nullptr && item->seq == seq && item->element.sensor_id == sensor_id && item->element.value == value;
}

// What the generated headers state about their entries is checked when this file
// is compiled: a number the template computes wrongly stops the build.
//
// The literal log: `<sce:retention entries="3"/>`, so a slot per entry kept and
// one more, 3 + 1 = 4. The element is an 8-bit id and a 16-bit value, 1 + 2 = 3
// bytes, so an entry is one length word and ceil(3 / 8) = 1 word of bytes:
// 1 + (3 + 7) / 8 = 1 + 1 = 2.
static_assert(literal::RETENTION == 3, "the retention is the document's");
static_assert(literal::SLOTS == 4, "one slot more than the retention");
static_assert(literal::MAX_ENCODED_BYTES == 3, "an 8-bit id and a 16-bit value are 1 + 2 bytes");
static_assert(literal::WORDS == 2, "one length word and one word of bytes");
static_assert(Log::kRetention == literal::RETENTION, "the runtime's retention is the header's");

// The deployed log: the deploy gives machine mcu_node `event_retention: 5`, so
// 5 + 1 = 6 slots. The element is a one-byte tag and a payload of at most eight
// bytes, 1 + 8 = 9 bytes, so an entry is one length word and ceil(9 / 8) = 2
// words of bytes: 1 + (9 + 7) / 8 = 1 + 2 = 3.
static_assert(deployed::RETENTION == 5, "the retention is the deploy's limit");
static_assert(deployed::SLOTS == 6, "one slot more than the retention");
static_assert(deployed::MAX_ENCODED_BYTES == 9, "a one-byte tag and at most eight payload bytes");
static_assert(deployed::WORDS == 3, "one length word and two words of bytes");
static_assert(DeployedLog::kRetention == deployed::RETENTION, "the runtime's retention is the header's");

// ─── The contract the headers state ───

void the_headers_state_what_the_documents_required() {
    CHECK(literal::DECLARED_PROGRESS == "wait-free", "the literal log's declared progress");
    CHECK(deployed::DECLARED_PROGRESS == "wait-free", "the deployed log's declared progress");
}

// ─── The log it defines is the log the runtime has ───

void entries_come_back_whole_and_in_order_through_the_typed_functions() {
    Log log;
    std::optional<literal::BroadcastLogConformanceLiteralPublisher> publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    literal::BroadcastLogConformanceLiteralReader reader = log.reader_oldest();
    CHECK(std::holds_alternative<Empty>(literal::read(log, reader)), "a new log is empty");

    // Values that fill every byte of the packed word and differ in each.
    const Event sent[] = {event(1, 0x0102), event(2, 0xFFEE), event(255, 0)};
    for (std::uint64_t n = 0; n < 3; ++n) {
        const literal::BroadcastLogConformanceLiteralPublished published = literal::publish(*publisher, sent[n]);
        CHECK(published.published() && !published.error.has_value() && published.seq == n,
              "an entry takes the next sequence number");
    }
    for (std::uint64_t n = 0; n < 3; ++n) {
        const Read got = literal::read(log, reader);
        CHECK(is_item(got, n, sent[n].sensor_id, sent[n].value), "an entry comes back whole, in order");
    }
    CHECK(std::holds_alternative<Empty>(literal::read(log, reader)), "drained");
    CHECK(reader.lost() == 0, "a reader that kept up lost nothing");
}

void an_entry_holds_its_length_and_its_bytes_little_endian_eight_to_a_word() {
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    // sensor_id 1 at byte 0 and value 0x0102 little endian at bytes 1 and 2: the
    // bytes 0x01, 0x02, 0x01, so the first word of bytes is
    // 0x01 + (0x02 << 8) + (0x01 << 16) = 0x010201.
    const literal::BroadcastLogConformanceLiteralPublished published = literal::publish(*publisher, event(1, 0x0102));
    CHECK(published.published() && published.seq == 0, "the entry takes sequence 0");

    // The runtime row is the one a reader of the typed `read` goes through, so what
    // it holds is what the header packed.
    auto reader = log.reader_oldest();
    Log::Entry entry{};
    const broadcast::Read answer = log.read(reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 0, "the runtime returns the entry");
    CHECK(entry[0] == 3, "the first word is the length of the encoded element");
    CHECK(entry[1] == 0x010201, "the bytes follow, little endian within the word");
}

void a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest() {
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto slow = log.reader_oldest();

    // Six entries, sequence numbers 0 to 5, into a log that retains three: the
    // window is [6 - 3, 6) = [3, 6), so the reader at 0 missed 3 - 0 = 3.
    for (std::uint16_t n = 0; n < 6; ++n) {
        const auto published = literal::publish(*publisher, event(9, n));
        CHECK(published.published(), "an entry within the bound is published");
    }
    const Read lapped = literal::read(log, slow);
    const Lagged *lag = std::get_if<Lagged>(&lapped);
    CHECK(lag != nullptr && lag->skipped == 3, "the reader is told it missed 3");
    CHECK(slow.cursor() == 3, "it resumes at the oldest entry held");
    CHECK(slow.lost() == 3, "and keeps the total");

    for (std::uint16_t n = 3; n < 6; ++n) {
        CHECK(is_item(literal::read(log, slow), n, 9, n), "what is held is read whole");
    }
    CHECK(std::holds_alternative<Empty>(literal::read(log, slow)), "then nothing is new");
    CHECK(slow.lost() == 3, "reading what is held loses nothing more");
}

void two_readers_keep_separate_cursors() {
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    CHECK(literal::publish(*publisher, event(1, 10)).seq == 0, "sequence 0");
    CHECK(literal::publish(*publisher, event(2, 20)).seq == 1, "sequence 1");

    // Opened at the oldest entry held (0, as nothing was forgotten), at sequence 1
    // and at the head.
    auto first = log.reader_oldest();
    auto second = log.reader_at(1);
    auto late = log.reader_head();

    CHECK(is_item(literal::read(log, first), 0, 1, 10), "the first reader takes sequence 0");
    CHECK(first.cursor() == 1, "its cursor moved");
    CHECK(second.cursor() == 1, "reading with one reader moves no other");

    CHECK(is_item(literal::read(log, second), 1, 2, 20), "the second reader takes sequence 1");
    CHECK(is_item(literal::read(log, first), 1, 2, 20), "the entry the second took is still there for the first");
    CHECK(std::holds_alternative<Empty>(literal::read(log, first)), "the first has read everything");
    CHECK(std::holds_alternative<Empty>(literal::read(log, second)), "the second has read everything");

    // A reader opened at the head sees only what is published after it.
    CHECK(std::holds_alternative<Empty>(literal::read(log, late)), "nothing is new for the late reader");
    CHECK(literal::publish(*publisher, event(3, 30)).seq == 2, "sequence 2");
    CHECK(is_item(literal::read(log, late), 2, 3, 30), "the late reader sees the new entry");
}

// ─── What the element's bound does ───

void a_record_larger_than_its_bound_is_refused_by_the_codec_and_not_truncated() {
    DeployedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");

    // 1 + 9 = 10 bytes is one past the entry's 9, and 1 + 20 = 21 is well past.
    for (const std::size_t length : {std::size_t{9}, std::size_t{20}}) {
        const Record too_long{.tag = 1, .payload = std::vector<std::uint8_t>(length, 0xAA)};
        const deployed::BroadcastLogConformanceDeployedPublished refused = deployed::publish(*publisher, too_long);
        CHECK(!refused.published(), "a payload past its bound is not published");
        CHECK(refused.error.has_value() && *refused.error == ::SCE::Forge::CodecError::BufferOverflow,
              "the bounded sink refuses it");
        CHECK(refused.seq == broadcast::kNoSequence, "a refused element has no sequence number");
        CHECK(log.head() == 0, "a refused element publishes nothing");
    }

    // Exactly the bound, and under it: the length word keeps what was written.
    const std::vector<std::uint8_t> full{1, 2, 3, 4, 5, 6, 7, 8};
    const std::vector<std::uint8_t> short_payload{7, 7, 7};
    const Record at_bound{.tag = 0x5A, .payload = full};
    const Record under_bound{.tag = 0x5B, .payload = short_payload};
    const auto first = deployed::publish(*publisher, at_bound);
    CHECK(first.published() && first.seq == 0, "a record of exactly 9 bytes is published");
    const auto second = deployed::publish(*publisher, under_bound);
    CHECK(second.published() && second.seq == 1, "a shorter record is published");

    // The bytes of the first record are tag 0x5A and 1 to 8: 9 bytes, so its entry
    // holds the length 9, the first eight bytes little endian in one word
    // (byte 0 lowest: 0x070605040302015A) and the ninth byte in the next (8).
    auto raw_reader = log.reader_oldest();
    DeployedLog::Entry entry{};
    const broadcast::Read answer = log.read(raw_reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 0, "the runtime returns the first record");
    CHECK(entry[0] == 9, "the length word is the encoded length");
    CHECK(entry[1] == 0x070605040302015AULL, "eight bytes fill the first word, the first byte lowest");
    CHECK(entry[2] == 8, "the ninth byte opens the next word");

    // The typed read hands back the records as they were published: the decoded
    // value owns its payload, so it needs no buffer of the reader's.
    auto reader = log.reader_oldest();
    const DeployedRead a = deployed::read(log, reader);
    const DeployedItem *record_a = std::get_if<DeployedItem>(&a);
    CHECK(record_a != nullptr && record_a->seq == 0 && record_a->element.tag == 0x5A &&
              record_a->element.payload == full,
          "the record of exactly 9 bytes comes back whole");
    const DeployedRead b = deployed::read(log, reader);
    const DeployedItem *record_b = std::get_if<DeployedItem>(&b);
    CHECK(record_b != nullptr && record_b->seq == 1 && record_b->element.tag == 0x5B &&
              record_b->element.payload == short_payload,
          "the shorter record comes back whole");
    CHECK(std::holds_alternative<DeployedEmpty>(deployed::read(log, reader)), "drained");
}

void a_stored_length_above_the_bound_is_malformed_and_never_a_copy_out_of_bounds() {
    // The runtime row takes any words, so an entry whose length word is not one this
    // log wrote (a corrupt slot, a log of another size) can be offered to the typed
    // read. A length of MAX_ENCODED_BYTES + 1 = 4 is the nearest; the largest
    // 64-bit value is the one a narrowing cast would wrap.
    const std::uint64_t lengths[] = {literal::MAX_ENCODED_BYTES + 1, UINT64_MAX};
    for (const std::uint64_t stored : lengths) {
        Log log;
        auto publisher = log.publisher();
        CHECK(publisher.has_value(), "the first publisher");
        Log::Entry entry{};
        entry[0] = stored;
        CHECK(publisher->publish(entry) == 0, "the runtime row takes the words");

        auto reader = log.reader_oldest();
        const Read got = literal::read(log, reader);
        const Malformed *bad = std::get_if<Malformed>(&got);
        CHECK(bad != nullptr && bad->seq == 0, "a length above the bound is malformed");
        CHECK(reader.cursor() == 1, "the reader moved past the entry");
    }
}

void an_entry_whose_bytes_are_no_element_is_malformed() {
    // A length of 1 is within the bound, but the event needs 3 bytes to decode.
    Log log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    Log::Entry entry{};
    entry[0] = 1;
    entry[1] = 0x01;
    CHECK(publisher->publish(entry) == 0, "the runtime row takes the words");

    auto reader = log.reader_oldest();
    const Read got = literal::read(log, reader);
    const Malformed *bad = std::get_if<Malformed>(&got);
    CHECK(bad != nullptr && bad->seq == 0, "bytes the codec cannot decode are malformed");
    CHECK(reader.cursor() == 1, "the reader moved past the entry");
}

}  // namespace

int main() {
    the_headers_state_what_the_documents_required();
    entries_come_back_whole_and_in_order_through_the_typed_functions();
    an_entry_holds_its_length_and_its_bytes_little_endian_eight_to_a_word();
    a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest();
    two_readers_keep_separate_cursors();
    a_record_larger_than_its_bound_is_refused_by_the_codec_and_not_truncated();
    a_stored_length_above_the_bound_is_malformed_and_never_a_copy_out_of_bounds();
    an_entry_whose_bytes_are_no_element_is_malformed();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_generated_test: all checks passed");
    return 0;
}
