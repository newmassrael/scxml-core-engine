// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The broadcast-log kind's headers over an element that asks its caller for what its
// wire form does not carry (ADR 0062 section 4, "Caller inputs are part of the
// element"), compiled and used.
//
// The log stores each of those in the entry beside the bytes, `publish` takes it and
// the item `read` returns holds it, so that a reader decodes an entry under what it
// was published under and not under whatever it happens to hold. Two logs, each
// generated from the documents of tests/forge/conformance/broadcast_log_context/ by
// `sce-codegen orchestrate` (every backend's conformance lane reads the same files):
//
//   broadcast_log_conformance_flagged  an element with one flag input that gates its
//                                      16-bit value: one byte, or three
//   broadcast_log_conformance_chosen   a variant whose wire form carries no tag: the
//                                      caller gives the tag that selects the arm (tag
//                                      1 the one-byte arm, any other tag the two-byte
//                                      default arm)
//
// What the header adds is the context word between the length and the bytes, so the
// words of an entry are read through the runtime and not through the typed read, and
// the typed read is then held to the flag and the tag it gives back. The generated
// headers are held to C++17 by the interface-standard gate (docs/adr/0059, section 7);
// this file is SCE's own source.

#include "broadcast_log_conformance_chosen.h"
#include "broadcast_log_conformance_flagged.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
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
using FlaggedFlags = flagged::BroadcastLogConformanceFlaggedFlags;
using FlaggedItem = flagged::BroadcastLogConformanceFlaggedItem;
using FlaggedEmpty = flagged::BroadcastLogConformanceFlaggedEmpty;
using FlaggedLagged = flagged::BroadcastLogConformanceFlaggedLagged;
using FlaggedMalformed = flagged::BroadcastLogConformanceFlaggedMalformed;
using FlaggedRead = flagged::BroadcastLogConformanceFlaggedRead;

using ChosenLog = chosen::BroadcastLogConformanceChosen;
using ChosenItem = chosen::BroadcastLogConformanceChosenItem;
using ChosenEmpty = chosen::BroadcastLogConformanceChosenEmpty;
using ChosenRead = chosen::BroadcastLogConformanceChosenRead;

// What the generated headers state about their entries is checked when this file is
// compiled: a number the template computes wrongly stops the build.
//
// The flagged element is an 8-bit id and a gated 16-bit value: at most 3 bytes. An
// entry is one length word, one context word (one flag) and 1 word of bytes:
// 1 + 1 + ceil(3 / 8) = 3 words.
static_assert(flagged::MAX_ENCODED_BYTES == 3, "the flagged element encodes to at most 1 + 2 = 3 bytes");
static_assert(flagged::CONTEXT_WORDS == 1, "one flag input is one context word");
static_assert(flagged::WORDS == 3, "a length word, a context word and one word of bytes");
static_assert(flagged::RETENTION == 3 && flagged::SLOTS == 4, "the document's retention of three, and one more slot");
static_assert(FlaggedLog::kRetention == flagged::RETENTION, "the runtime's retention is the header's");

// The choice is the larger of its arms, two bytes; the tag is one context word.
static_assert(chosen::MAX_ENCODED_BYTES == 2, "the choice encodes to at most the larger arm: 2 bytes");
static_assert(chosen::CONTEXT_WORDS == 1, "a tag the caller gives is one context word");
static_assert(chosen::WORDS == 3, "a length word, a context word and one word of bytes");
static_assert(chosen::RETENTION == 3 && chosen::SLOTS == 4, "the document's retention of three, and one more slot");
static_assert(ChosenLog::kRetention == chosen::RETENTION, "the runtime's retention is the header's");

void the_headers_state_what_the_documents_required() {
    CHECK(flagged::DECLARED_PROGRESS == "wait-free", "the flagged log's declared progress");
    CHECK(chosen::DECLARED_PROGRESS == "wait-free", "the chosen log's declared progress");
}

// ─── A flag the codec takes from its caller ───

FlaggedElement make_event(std::uint8_t sensor_id, bool present, std::uint16_t value) {
    FlaggedElement element{};
    element.sensor_id = sensor_id;
    if (present) {
        element.value = value;
    }
    return element;
}

FlaggedFlags flags_of(bool has_value) {
    FlaggedFlags flags;
    flags.has_value = has_value;
    return flags;
}

std::uint64_t publish_flagged(flagged::BroadcastLogConformanceFlaggedPublisher &publisher,
                              const FlaggedElement &element, bool has_value) {
    const flagged::BroadcastLogConformanceFlaggedPublished published =
        flagged::publish(publisher, element, flags_of(has_value));
    CHECK(published.published() && !published.error.has_value(),
          "an element within its bound is published under its flag");
    return published.seq;
}

/// Whether `got` is the entry `seq`: `want`, whole, with the flag it was published under.
bool is_flagged_item(const FlaggedRead &got, std::uint64_t seq, const FlaggedElement &want, bool has_value) {
    const FlaggedItem *item = std::get_if<FlaggedItem>(&got);
    if (item == nullptr || item->seq != seq || item->flags.has_value != has_value) {
        return false;
    }
    if (item->element.sensor_id != want.sensor_id || item->element.value.has_value() != want.value.has_value()) {
        return false;
    }
    return !want.value.has_value() || *item->element.value == *want.value;
}

void the_context_is_in_the_word_after_the_length_and_the_bytes_follow_it() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    const FlaggedElement first = make_event(1, true, 0x0203);
    const FlaggedElement second = make_event(2, false, 0);
    CHECK(publish_flagged(*publisher, first, true) == 0, "the first entry takes sequence 0");
    CHECK(publish_flagged(*publisher, second, false) == 1, "the second entry takes sequence 1");

    auto reader = log.reader_oldest();
    FlaggedLog::Entry entry{};
    broadcast::Read answer = log.read(reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 0, "the runtime returns the first entry");
    // Three bytes: 01 03 02 (the id, then the value little endian), which in a word
    // are 0x01 + 0x03 * 2^8 + 0x02 * 2^16 = 0x020301. The flag is bit 0 of the context word.
    CHECK(entry[0] == 3 && entry[1] == 1 && entry[2] == 0x020301,
          "a three-byte entry under a set flag is [3, 1, 0x020301]");
    entry = FlaggedLog::Entry{};
    answer = log.read(reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 1, "the runtime returns the second entry");
    // One byte, under a flag that is not set.
    CHECK(entry[0] == 1 && entry[1] == 0 && entry[2] == 0x02, "a one-byte entry under a clear flag is [1, 0, 0x02]");
}

void a_reader_gets_back_the_flag_an_entry_was_published_under() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto reader = log.reader_oldest();
    CHECK(std::holds_alternative<FlaggedEmpty>(flagged::read(log, reader)), "a new log has nothing to read");

    // The same element shape, published under each answer of the flag: the value is on
    // the wire under a set flag and absent under a clear one, so the two frames are
    // three bytes and one, and neither can be decoded under the other's flag.
    const FlaggedElement sent[2] = {make_event(1, true, 0x0203), make_event(2, false, 0)};
    const bool flags[2] = {true, false};
    for (std::uint64_t n = 0; n < 2; ++n) {
        CHECK(publish_flagged(*publisher, sent[n], flags[n]) == n, "an entry takes the next sequence number");
    }
    for (std::uint64_t n = 0; n < 2; ++n) {
        CHECK(is_flagged_item(flagged::read(log, reader), n, sent[n], flags[n]),
              "an entry comes back whole under the flag it was published under");
    }
    CHECK(std::holds_alternative<FlaggedEmpty>(flagged::read(log, reader)), "both were read");
}

void every_reader_of_an_entry_decodes_it_under_the_same_flag() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto early = log.reader_oldest();
    auto late = log.reader_oldest();
    const FlaggedElement first = make_event(5, true, 500);
    const FlaggedElement second = make_event(6, false, 0);
    (void)publish_flagged(*publisher, first, true);
    (void)publish_flagged(*publisher, second, false);
    CHECK(is_flagged_item(flagged::read(log, early), 0, first, true), "the early reader, entry 0");
    CHECK(is_flagged_item(flagged::read(log, early), 1, second, false), "the early reader, entry 1");
    CHECK(is_flagged_item(flagged::read(log, late), 0, first, true), "the late reader, entry 0");
    CHECK(is_flagged_item(flagged::read(log, late), 1, second, false), "the late reader, entry 1");
}

void a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under() {
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto slow = log.reader_oldest();

    // Five entries into a log that retains three: the window is [2, 5). The flag
    // alternates, so a flag that stayed with the wrong slot would show.
    std::vector<FlaggedElement> sent;
    for (std::uint8_t n = 0; n < 5; ++n) {
        const bool has_value = (n % 2) == 0;
        sent.push_back(make_event(static_cast<std::uint8_t>(n + 1), has_value, static_cast<std::uint16_t>(100 + n)));
        (void)publish_flagged(*publisher, sent.back(), has_value);
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

void the_flag_that_was_stored_decides_how_an_entry_is_read_and_the_log_does_not_judge_it() {
    // The C++ codec does not check a flag input against the field it gates when it
    // encodes (it writes a gated field whenever the value is present, and only the
    // chain-membership predicates are checked), and the log does not either: the
    // publisher gives what it knows. A codec that refused a disagreement would refuse
    // the first publish here, and this test would turn red there.
    FlaggedLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto typed = log.reader_oldest();
    auto raw = log.reader_oldest();

    // The value is there and the flag says it is not: the three bytes are stored under
    // a flag that is clear, which the entry's context word says.
    const flagged::BroadcastLogConformanceFlaggedPublished kept =
        flagged::publish(*publisher, make_event(7, true, 1), flags_of(false));
    CHECK(kept.published() && !kept.error.has_value(), "the log does not judge the flag against the element");
    FlaggedLog::Entry entry{};
    const broadcast::Read answer = log.read(raw, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && entry[0] == 3 && entry[1] == 0,
          "the three bytes are stored under the clear flag the publisher gave");

    // The value is absent and the flag says it is there: the one byte stored is too few
    // for the value the flag promises, so the codec cannot decode the entry.
    const flagged::BroadcastLogConformanceFlaggedPublished short_entry =
        flagged::publish(*publisher, make_event(8, false, 0), flags_of(true));
    CHECK(short_entry.published() && !short_entry.error.has_value(),
          "an absent value under a set flag is stored as it was given");
    // The reader passes the first entry (published under a clear flag) and is then at the
    // second.
    (void)flagged::read(log, typed);
    const FlaggedRead got = flagged::read(log, typed);
    const FlaggedMalformed *bad = std::get_if<FlaggedMalformed>(&got);
    CHECK(bad != nullptr && bad->seq == 1, "the codec refuses the entry its stored flag cannot decode");
    CHECK(typed.cursor() == 2, "the reader moved past the entry");
}

// ─── The tag of a variant that has none of its own ───

/// The element whose arm `tag` selects out of `bytes`, which is how the codec makes one:
/// the wire form carries no tag, so the caller gives it on decode.
ChoiceElement choice_from(const std::vector<std::uint8_t> &bytes, std::uint8_t tag) {
    ::SCE::Forge::SceCursor cursor(bytes.data(), bytes.size());
    std::optional<ChoiceElement> element = ChoiceElement::decode(cursor, tag);
    CHECK(element.has_value(), "the arm bytes decode under the tag");
    return element.has_value() ? std::move(*element) : ChoiceElement{};
}

/// The bytes of `element` as its codec writes them: the arm's own bytes only.
std::vector<std::uint8_t> choice_bytes(const ChoiceElement &element) {
    std::array<std::uint8_t, 8> buffer{};
    ::SCE::Forge::SpanSink sink(buffer.data(), buffer.size());
    const std::optional<::SCE::Forge::CodecError> refused = element.encode(sink);
    CHECK(!refused.has_value(), "the element encodes");
    return std::vector<std::uint8_t>(buffer.begin(), buffer.begin() + static_cast<std::ptrdiff_t>(sink.position()));
}

std::uint64_t publish_chosen(chosen::BroadcastLogConformanceChosenPublisher &publisher, const ChoiceElement &element,
                             std::uint8_t tag) {
    const chosen::BroadcastLogConformanceChosenPublished published = chosen::publish(publisher, element, tag);
    CHECK(published.published() && !published.error.has_value(),
          "an element within its bound is published under its tag");
    return published.seq;
}

/// Whether `got` is the entry `seq`: an element that encodes to the same bytes as `want`,
/// which for these arms is the same arm, with the tag it was published with.
bool is_chosen_item(const ChosenRead &got, std::uint64_t seq, const std::vector<std::uint8_t> &want, std::uint8_t tag) {
    const ChosenItem *item = std::get_if<ChosenItem>(&got);
    return item != nullptr && item->seq == seq && item->tag == tag && choice_bytes(item->element) == want;
}

void the_tag_is_in_the_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold() {
    ChosenLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    const ChoiceElement small = choice_from({9}, 1);
    const ChoiceElement large = choice_from({0x02, 0x01}, 0);
    CHECK(publish_chosen(*publisher, small, 1) == 0, "the first entry takes sequence 0");
    CHECK(publish_chosen(*publisher, large, 0) == 1, "the second entry takes sequence 1");

    auto reader = log.reader_oldest();
    ChosenLog::Entry entry{};
    broadcast::Read answer = log.read(reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 0, "the runtime returns the first entry");
    CHECK(entry[0] == 1 && entry[1] == 1 && entry[2] == 9, "the small arm under tag 1 is [1, 1, 9]");
    entry = ChosenLog::Entry{};
    answer = log.read(reader, entry);
    CHECK(answer.kind == broadcast::ReadKind::Item && answer.seq == 1, "the runtime returns the second entry");
    CHECK(entry[0] == 2 && entry[1] == 0 && entry[2] == 0x0102, "the large arm under tag 0 is [2, 0, 0x0102]");
}

void a_reader_gets_back_the_tag_an_entry_was_published_under() {
    ChosenLog log;
    auto publisher = log.publisher();
    CHECK(publisher.has_value(), "the first publisher");
    auto reader = log.reader_oldest();
    CHECK(std::holds_alternative<ChosenEmpty>(chosen::read(log, reader)), "a new log has nothing to read");

    // The wire form of an arm carries no tag, so the two arms' bytes are the arm's own and
    // which arm a frame is comes from the tag stored beside it.
    const std::vector<std::uint8_t> small_nine{9};
    const std::vector<std::uint8_t> large_bytes{0x02, 0x01};
    const std::vector<std::uint8_t> small_max{255};
    CHECK(publish_chosen(*publisher, choice_from(small_nine, 1), 1) == 0, "publish 0 takes sequence number 0");
    CHECK(publish_chosen(*publisher, choice_from(large_bytes, 0), 0) == 1, "publish 1 takes sequence number 1");
    CHECK(publish_chosen(*publisher, choice_from(small_max, 1), 1) == 2, "publish 2 takes sequence number 2");
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
    the_headers_state_what_the_documents_required();
    the_context_is_in_the_word_after_the_length_and_the_bytes_follow_it();
    a_reader_gets_back_the_flag_an_entry_was_published_under();
    every_reader_of_an_entry_decodes_it_under_the_same_flag();
    a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under();
    the_flag_that_was_stored_decides_how_an_entry_is_read_and_the_log_does_not_judge_it();
    the_tag_is_in_the_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold();
    a_reader_gets_back_the_tag_an_entry_was_published_under();

    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("broadcast_log_context_generated_test: all checks passed");
    return 0;
}
