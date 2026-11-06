// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast-log kind's narrow-row headers over an element that asks its caller for what its
// wire form does not carry (ADR 0062 section 4, "Caller inputs are part of the element"),
// compiled and used.
//
// The same two logs as broadcast_log_context_generated_test.cpp, generated from the documents
// of tests/forge/conformance/broadcast_log_context/ by `sce-codegen orchestrate` for the machine
// of a deploy that names a target with 32-bit atomics and states the pause the design relies on,
// so the generator writes the narrow row: 32-bit words and a stamp of 32 bits.
//
//   broadcast_log_conformance_flagged  an element with one flag input that gates its 16-bit
//                                      value: one byte, or three
//   broadcast_log_conformance_chosen   a variant whose wire form carries no tag, which the log
//                                      takes from the element's `tag()`
//
// What the narrow header adds to the 64-bit one is the word: the context word between the length
// and the bytes is a 32-bit word, and the bytes are four to a word. The words of an entry are
// read through the runtime, and the typed read is then held to the flag and the tag it gives
// back. A flag is a bit of a context word of 32 bits (flag `i` is bit `i % 32` of word `i / 32`,
// which a text test of sce-build holds for 33 flags, as no document of this directory has more
// than one). The generated headers are held to C++17 by the interface-standard gate
// (docs/adr/0059, section 7); this file is SCE's own source.

#include "broadcast_log_conformance_chosen.h"
#include "broadcast_log_conformance_flagged.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <type_traits>
#include <utility>
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
namespace flagged = ::SCE::Generated::BroadcastLogConformanceFlagged;
namespace chosen = ::SCE::Generated::BroadcastLogConformanceChosen;

using FlaggedElement = ::SCE::Generated::BroadcastLogConformanceFlaggedEvent::BroadcastLogConformanceFlaggedEvent;
using ChoiceElement = ::SCE::Generated::BroadcastLogConformanceChoice::BroadcastLogConformanceChoice;

using FlaggedLog = flagged::BroadcastLogConformanceFlagged;
using FlaggedItem = flagged::BroadcastLogConformanceFlaggedItem;
using FlaggedEmpty = flagged::BroadcastLogConformanceFlaggedEmpty;
using FlaggedLagged = flagged::BroadcastLogConformanceFlaggedLagged;
using FlaggedRead = flagged::BroadcastLogConformanceFlaggedRead;

using ChosenLog = chosen::BroadcastLogConformanceChosen;
using ChosenItem = chosen::BroadcastLogConformanceChosenItem;
using ChosenEmpty = chosen::BroadcastLogConformanceChosenEmpty;
using ChosenRead = chosen::BroadcastLogConformanceChosenRead;

// What the generated headers state about their entries is checked when this file is compiled: a
// number the template computes wrongly stops the build.
//
// The flagged element is an 8-bit id and a gated 16-bit value: at most 3 bytes. An entry is one
// length word, one context word (one flag) and ceil(3 / 4) = 1 word of bytes: 1 + 1 + 1 = 3
// words of 32 bits.
static_assert(flagged::MAX_ENCODED_BYTES == 3, "the flagged element encodes to at most 1 + 2 = 3 bytes");
static_assert(flagged::CONTEXT_WORDS == 1, "one flag input is one context word");
static_assert(flagged::WORDS == 3, "a length word, a context word and one word of bytes");
static_assert(flagged::STAMP_BITS == 32, "the flagged log was generated over the narrow row");
static_assert(flagged::RETENTION == 3 && flagged::SLOTS == 4, "the document's retention of three, and one more slot");
static_assert(std::is_same<FlaggedLog::Entry, std::array<std::uint32_t, flagged::WORDS>>::value,
              "the words of the flagged entry are 32-bit");

// The choice is the larger of its arms, two bytes; the tag is one context word.
static_assert(chosen::MAX_ENCODED_BYTES == 2, "the choice encodes to at most the larger arm: 2 bytes");
static_assert(chosen::CONTEXT_WORDS == 1, "a tag the entry stores is one context word");
static_assert(chosen::WORDS == 3, "a length word, a context word and one word of bytes");
static_assert(chosen::STAMP_BITS == 32, "the chosen log was generated over the narrow row");
static_assert(std::is_same<ChosenLog::Entry, std::array<std::uint32_t, chosen::WORDS>>::value,
              "the words of the chosen entry are 32-bit");

// ─── A flag the codec's decode takes from its caller ───

FlaggedElement make_event(std::uint8_t sensor_id, bool present, std::uint16_t value) {
    return FlaggedElement{sensor_id, present ? std::optional<std::uint16_t>{value} : std::nullopt};
}

// Publish `element`. The value says the flag, so `publish` takes none and stores the one the
// element says (ADR 0065 section 6, R4a).
std::uint64_t publish_flagged(flagged::BroadcastLogConformanceFlaggedPublisher &publisher,
                              const FlaggedElement &element) {
    const flagged::BroadcastLogConformanceFlaggedPublished published = flagged::publish(publisher, element);
    CHECK(published.published() && !published.error.has_value(),
          "an element within its bound is published under its flag");
    return published.seq;
}

// Whether `got` is the entry `seq`: `want`, whole, with the flag it was published under.
bool is_flagged_item(const FlaggedRead &got, std::uint64_t seq, const FlaggedElement &want, bool has_value) {
    const FlaggedItem *item = std::get_if<FlaggedItem>(&got);
    if (item == nullptr || item->seq != seq || item->flags.has_value != has_value) {
        return false;
    }
    if (item->element.sensor_id() != want.sensor_id() ||
        item->element.value().has_value() != want.value().has_value()) {
        return false;
    }
    return !want.value().has_value() || *item->element.value() == *want.value();
}

void the_context_is_a_32_bit_word_after_the_length_and_the_bytes_follow_it() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    CHECK(publish_flagged(*publisher, make_event(1, true, 0x0203)) == 0, "the first entry takes sequence 0");
    CHECK(publish_flagged(*publisher, make_event(2, false, 0)) == 1, "the second entry takes sequence 1");

    auto reader = log.reader_oldest();
    FlaggedLog::Entry entry{};
    broadcast::Read answer = log.read(reader, entry);
    CHECK(answer == broadcast::Read::item(0), "the runtime returns the first entry");
    // Three bytes: 01 03 02 (the id, then the value little endian), which in a word are
    // 0x01 + 0x03 * 2^8 + 0x02 * 2^16 = 0x020301. The flag is bit 0 of the context word.
    CHECK(entry[0] == 3 && entry[1] == 1 && entry[2] == 0x020301,
          "a three-byte entry under a set flag is [3, 1, 0x020301]");
    entry = FlaggedLog::Entry{};
    answer = log.read(reader, entry);
    CHECK(answer == broadcast::Read::item(1), "the runtime returns the second entry");
    // One byte, under a flag that is not set.
    CHECK(entry[0] == 1 && entry[1] == 0 && entry[2] == 0x02, "a one-byte entry under a clear flag is [1, 0, 0x02]");
}

void a_reader_gets_back_the_flag_an_entry_was_published_under() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto reader = log.reader_oldest();
    CHECK(std::holds_alternative<FlaggedEmpty>(flagged::read(log, reader)), "a new log has nothing to read");

    // The same element shape, published under each answer of the flag: the value is on the wire
    // under a set flag and absent under a clear one, so the two frames are three bytes and one,
    // and neither can be decoded under the other's flag. The value says the flag, so `publish`
    // takes none (ADR 0065 section 6, R4a).
    const FlaggedElement sent[2] = {make_event(1, true, 0x0203), make_event(2, false, 0)};
    const bool flags[2] = {true, false};
    for (std::uint64_t n = 0; n < 2; ++n) {
        CHECK(publish_flagged(*publisher, sent[n]) == n, "an entry takes the next sequence number");
    }
    for (std::uint64_t n = 0; n < 2; ++n) {
        CHECK(is_flagged_item(flagged::read(log, reader), n, sent[n], flags[n]),
              "an entry comes back whole under the flag it was published under");
    }
    CHECK(std::holds_alternative<FlaggedEmpty>(flagged::read(log, reader)), "both were read");
}

void a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto slow = log.reader_oldest();

    // Five entries into a log that retains three: the window is [2, 5). The flag alternates, so a
    // flag that stayed with the wrong slot would show.
    std::vector<FlaggedElement> sent;
    for (std::uint8_t n = 0; n < 5; ++n) {
        const bool has_value = (n % 2) == 0;
        sent.push_back(make_event(static_cast<std::uint8_t>(n + 1), has_value, static_cast<std::uint16_t>(100 + n)));
        (void)publish_flagged(*publisher, sent.back());
    }
    const FlaggedRead lapped = flagged::read(log, slow);
    const FlaggedLagged *lag = std::get_if<FlaggedLagged>(&lapped);
    CHECK(lag != nullptr && lag->skipped == 2, "the reader is told it missed two");
    for (std::uint64_t n = 2; n < 5; ++n) {
        CHECK(is_flagged_item(flagged::read(log, slow), n, sent[n], (n % 2) == 0),
              "what is held is read whole, under its own flag");
    }
    CHECK(std::holds_alternative<FlaggedEmpty>(flagged::read(log, slow)), "nothing is left after the window");
}

void an_entry_is_stored_under_the_flag_its_element_says() {
    // The flag is read off the element, so the log cannot hold an entry whose bytes and flag
    // disagree: there is no flag to give that could (ADR 0065 section 6, R4a).
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto reader = log.reader_oldest();

    const FlaggedElement present = make_event(7, true, 1);
    const FlaggedElement absent = make_event(7, false, 0);
    CHECK(publish_flagged(*publisher, present) == 0, "a present value is published as entry 0");
    CHECK(is_flagged_item(flagged::read(log, reader), 0, present, true), "it is stored under a set flag");
    CHECK(publish_flagged(*publisher, absent) == 1, "an absent value is published as entry 1");
    CHECK(is_flagged_item(flagged::read(log, reader), 1, absent, false), "it is stored under a clear flag");
}

// ─── The tag of a variant that has none of its own ───

// The element whose arm `tag` selects out of `bytes`, which is how the codec makes one: the wire
// form carries no tag, so the caller gives it on decode.
ChoiceElement choice_from(const std::vector<std::uint8_t> &bytes, std::uint8_t tag) {
    ::SCE::Forge::SceCursor cursor(bytes.data(), bytes.size());
    std::optional<ChoiceElement> element = ChoiceElement::decode(cursor, tag);
    CHECK(element.has_value(), "the arm bytes decode under the tag");
    return element.has_value() ? std::move(*element) : ChoiceElement{};
}

// The bytes of `element` as its codec writes them: the arm's own bytes only.
std::vector<std::uint8_t> choice_bytes(const ChoiceElement &element) {
    std::array<std::uint8_t, 8> buffer{};
    ::SCE::Forge::SpanSink sink(buffer.data(), buffer.size());
    const std::optional<::SCE::Forge::CodecError> refused = element.encode(sink);
    CHECK(!refused.has_value(), "the element encodes");
    return std::vector<std::uint8_t>(buffer.begin(), buffer.begin() + static_cast<std::ptrdiff_t>(sink.position()));
}

std::uint64_t publish_chosen(chosen::BroadcastLogConformanceChosenPublisher &publisher, const ChoiceElement &element) {
    const chosen::BroadcastLogConformanceChosenPublished published = chosen::publish(publisher, element);
    CHECK(published.published() && !published.error.has_value(),
          "an element within its bound is published under the tag of its arm");
    return published.seq;
}

// Whether `got` is the entry `seq`: an element that encodes to the same bytes as `want`, which
// for these arms is the same arm, and that reports `tag` as its own.
bool is_chosen_item(const ChosenRead &got, std::uint64_t seq, const std::vector<std::uint8_t> &want, std::uint8_t tag) {
    const ChosenItem *item = std::get_if<ChosenItem>(&got);
    return item != nullptr && item->seq == seq && item->element.tag() == tag && choice_bytes(item->element) == want;
}

void the_tag_is_in_a_32_bit_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold() {
    ChosenLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    CHECK(publish_chosen(*publisher, choice_from({9}, 1)) == 0, "the first entry takes sequence 0");
    CHECK(publish_chosen(*publisher, choice_from({0x02, 0x01}, 0)) == 1, "the second entry takes sequence 1");

    auto reader = log.reader_oldest();
    ChosenLog::Entry entry{};
    broadcast::Read answer = log.read(reader, entry);
    CHECK(answer == broadcast::Read::item(0), "the runtime returns the first entry");
    CHECK(entry[0] == 1 && entry[1] == 1 && entry[2] == 9, "the small arm under tag 1 is [1, 1, 9]");
    entry = ChosenLog::Entry{};
    answer = log.read(reader, entry);
    CHECK(answer == broadcast::Read::item(1), "the runtime returns the second entry");
    CHECK(entry[0] == 2 && entry[1] == 0 && entry[2] == 0x0102, "the large arm under tag 0 is [2, 0, 0x0102]");
}

void a_reader_gets_back_the_tag_an_entry_was_published_under() {
    ChosenLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto reader = log.reader_oldest();
    CHECK(std::holds_alternative<ChosenEmpty>(chosen::read(log, reader)), "a new log has nothing to read");

    const std::vector<std::uint8_t> small_nine{9};
    const std::vector<std::uint8_t> large_bytes{0x02, 0x01};
    const std::vector<std::uint8_t> small_max{255};
    CHECK(publish_chosen(*publisher, choice_from(small_nine, 1)) == 0, "publish 0 takes sequence number 0");
    CHECK(publish_chosen(*publisher, choice_from(large_bytes, 0)) == 1, "publish 1 takes sequence number 1");
    CHECK(publish_chosen(*publisher, choice_from(small_max, 1)) == 2, "publish 2 takes sequence number 2");
    CHECK(is_chosen_item(chosen::read(log, reader), 0, small_nine, 1),
          "entry 0 comes back as the small arm under tag 1");
    CHECK(is_chosen_item(chosen::read(log, reader), 1, large_bytes, 0),
          "entry 1 comes back as the large arm under tag 0");
    CHECK(is_chosen_item(chosen::read(log, reader), 2, small_max, 1),
          "entry 2 comes back as the small arm under tag 1");
    CHECK(std::holds_alternative<ChosenEmpty>(chosen::read(log, reader)), "all three were read");
}

}  // namespace

int main() {
    the_context_is_a_32_bit_word_after_the_length_and_the_bytes_follow_it();
    a_reader_gets_back_the_flag_an_entry_was_published_under();
    a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under();
    an_entry_is_stored_under_the_flag_its_element_says();
    the_tag_is_in_a_32_bit_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold();
    a_reader_gets_back_the_tag_an_entry_was_published_under();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_narrow_context_generated_test: all checks passed");
    return 0;
}
