/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The broadcast-log kind's headers over an element that asks its caller for what
 * its wire form does not carry (ADR 0062 section 4, "Caller inputs are part of the
 * element"), compiled and used.
 *
 * The log stores each of those in the entry beside the bytes, `publish` takes it
 * and the read result returns it, so that a reader decodes an entry under what it
 * was published under and not under whatever it happens to hold. Two logs, each
 * generated from the documents of tests/forge/conformance/broadcast_log_context/ by
 * `sce-codegen orchestrate` (every backend's conformance lane reads the same
 * files):
 *
 *   broadcast_log_conformance_flagged  an element with one flag input that gates
 *                                      its 16-bit value: one byte, or three
 *   broadcast_log_conformance_chosen   a variant whose wire form carries no tag:
 *                                      the caller gives the tag that selects the
 *                                      arm (tag 1 the one-byte arm, any other tag
 *                                      the two-byte default arm)
 *
 * What the header adds is the context word between the length and the bytes, so
 * the words of an entry are read through the runtime and not through the typed
 * read, and the typed read is then held to the flag and the tag it gives back.
 */

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "broadcast_log_conformance_chosen.h"
#include "broadcast_log_conformance_flagged.h"

static unsigned g_failures = 0;
static unsigned g_checks = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        g_checks++;                                                                                                    \
        if (!(condition)) {                                                                                            \
            g_failures++;                                                                                              \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
        }                                                                                                              \
    } while (0)

/* A name of the generated log of each document: its lower-case identifiers and
 * its upper-case macros. */
#define FLG(NAME) broadcast_log_conformance_flagged_##NAME
#define FLG_U(NAME) BROADCAST_LOG_CONFORMANCE_FLAGGED_##NAME
#define CHS(NAME) broadcast_log_conformance_chosen_##NAME
#define CHS_U(NAME) BROADCAST_LOG_CONFORMANCE_CHOSEN_##NAME

typedef broadcast_log_conformance_flagged_event_t flagged_element_t;
typedef broadcast_log_conformance_choice_t choice_element_t;

/* ---- The slot: the length, the context words, then the bytes ---- */

static void the_headers_state_the_context_words_of_an_entry(void) {
    /* The flagged element is an 8-bit id and a gated 16-bit value: at most 3 bytes.
     * An entry is one length word, one context word (one flag) and 1 word of bytes:
     * 1 + 1 + ceil(3 / 8) = 3 words. */
    CHECK(FLG_U(MAX_ENCODED_BYTES) == 3u, "the flagged element encodes to at most 1 + 2 = 3 bytes");
    CHECK(FLG_U(CONTEXT_WORDS) == 1u, "one flag input is one context word");
    CHECK(FLG_U(WORDS) == 3u, "a length word, a context word and one word of bytes");
    CHECK(FLG_U(RETENTION) == 3u && FLG_U(SLOTS) == 4u, "the document's retention of three, and one more slot");
    CHECK(strcmp(FLG_U(DECLARED_PROGRESS), "wait-free") == 0, "the document declared wait-free");
    /* The storage is the slots, each a stamp and its words: 4 * (3 + 1) = 16. */
    CHECK(sizeof(((FLG(t) *)0)->storage) / sizeof(uint64_t) == 16u, "four slots of a stamp and three words");

    /* The choice is the larger of its arms, two bytes; the tag is one context word. */
    CHECK(CHS_U(MAX_ENCODED_BYTES) == 2u, "the choice encodes to at most the larger arm: 2 bytes");
    CHECK(CHS_U(CONTEXT_WORDS) == 1u, "a tag the caller gives is one context word");
    CHECK(CHS_U(WORDS) == 3u, "a length word, a context word and one word of bytes");
    CHECK(CHS_U(RETENTION) == 3u && CHS_U(SLOTS) == 4u, "the document's retention of three, and one more slot");
}

/* ---- A flag the codec takes from its caller ---- */

static flagged_element_t make_event(unsigned sensor, bool present, unsigned value) {
    flagged_element_t e;
    memset(&e, 0, sizeof e);
    e.sensor_id = (uint8_t)sensor;
    e.value.present = present;
    e.value.value = (uint16_t)(present ? value : 0u);
    return e;
}

static FLG(flags_t) flags_of(bool has_value) {
    FLG(flags_t) flags;
    flags.has_value = has_value;
    return flags;
}

static uint64_t publish_flagged(FLG(publisher_t) * publisher, const flagged_element_t *element, bool has_value) {
    const FLG(flags_t) flags = flags_of(has_value);
    const FLG(publish_result_t) result = FLG(publish)(publisher, element, &flags);
    CHECK(result.status == FLG_U(PUBLISH_OK), "an element within its bound is published under its flag");
    return result.seq;
}

/* The next read of `reader` is the entry `seq`: `want`, whole, with the flag it was
 * published under. */
static void expect_flagged(const FLG(t) * ring, FLG(reader_t) * reader, uint64_t seq, const flagged_element_t *want,
                           bool has_value) {
    flagged_element_t got;
    FLG(read_result_t) result;
    memset(&got, 0, sizeof got);
    result = FLG(read)(ring, reader, &got);
    CHECK(result.status == FLG_U(READ_ITEM), "entry %u: an item", (unsigned)seq);
    CHECK(result.seq == seq, "entry %u: it carries sequence number %u", (unsigned)seq, (unsigned)result.seq);
    CHECK(result.flags.has_value == has_value, "entry %u: it comes back under the flag it was published under",
          (unsigned)seq);
    CHECK(got.sensor_id == want->sensor_id, "entry %u: sensor %u", (unsigned)seq, (unsigned)got.sensor_id);
    CHECK(got.value.present == want->value.present, "entry %u: its value is present or not as sent", (unsigned)seq);
    if (want->value.present) {
        CHECK(got.value.value == want->value.value, "entry %u: its value %u", (unsigned)seq, (unsigned)got.value.value);
    }
}

/* The next read of `core` as the runtime gives it: the entry's words, whole. */
static void read_words(const sce_blog_t *core, sce_blog_reader_t *reader, uint64_t *words) {
    const sce_blog_read_t found = sce_blog_read(core, reader, words);
    CHECK(found.status == SCE_BLOG_ITEM, "the runtime hands back an entry");
}

static void the_context_is_in_the_word_after_the_length_and_the_bytes_follow_it(void) {
    static FLG(t) ring;
    FLG(publisher_t) publisher = {0};
    sce_blog_reader_t reader;
    uint64_t words[FLG_U(WORDS)];
    flagged_element_t first = make_event(1u, true, 0x0203u);
    flagged_element_t second = make_event(2u, false, 0u);
    CHECK(FLG(init)(&ring) && FLG(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    (void)publish_flagged(&publisher, &first, true);
    (void)publish_flagged(&publisher, &second, false);

    reader = sce_blog_reader_oldest(&ring.core);
    memset(words, 0xEE, sizeof words);
    read_words(&ring.core, &reader, words);
    /* Three bytes: 01 03 02 (the id, then the value little-endian), which little-endian
     * in a word are 0x01 + 0x03 * 2^8 + 0x02 * 2^16 = 0x020301. The flag is bit 0 of the
     * context word. */
    CHECK(words[0] == 3u && words[1] == 1u && words[2] == UINT64_C(0x020301),
          "a three-byte entry under a set flag is [3, 1, 0x020301], not [%u, %u, 0x%X]", (unsigned)words[0],
          (unsigned)words[1], (unsigned)words[2]);
    memset(words, 0xEE, sizeof words);
    read_words(&ring.core, &reader, words);
    /* One byte, under a flag that is not set. */
    CHECK(words[0] == 1u && words[1] == 0u && words[2] == UINT64_C(0x02),
          "a one-byte entry under a clear flag is [1, 0, 0x02], not [%u, %u, 0x%X]", (unsigned)words[0],
          (unsigned)words[1], (unsigned)words[2]);
    FLG(publisher_release)(&publisher);
}

static void a_reader_gets_back_the_flag_an_entry_was_published_under(void) {
    static FLG(t) ring;
    FLG(publisher_t) publisher = {0};
    FLG(reader_t) reader;
    FLG(read_result_t) empty;
    flagged_element_t got;
    /* The same element shape, published under each answer of the flag: the value is
     * on the wire under a set flag and absent under a clear one, so the two frames are
     * three bytes and one, and neither can be decoded under the other's flag. */
    const flagged_element_t sent[2] = {make_event(1u, true, 0x0203u), make_event(2u, false, 0u)};
    const bool flags[2] = {true, false};
    unsigned n;
    CHECK(FLG(init)(&ring) && FLG(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = FLG(reader_oldest)(&ring);
    memset(&got, 0, sizeof got);
    empty = FLG(read)(&ring, &reader, &got);
    CHECK(empty.status == FLG_U(READ_EMPTY), "a new log has nothing to read");

    for (n = 0; n < 2u; n++) {
        CHECK(publish_flagged(&publisher, &sent[n], flags[n]) == n, "publish %u takes sequence number %u", n, n);
    }
    for (n = 0; n < 2u; n++) {
        expect_flagged(&ring, &reader, n, &sent[n], flags[n]);
    }
    memset(&got, 0, sizeof got);
    empty = FLG(read)(&ring, &reader, &got);
    CHECK(empty.status == FLG_U(READ_EMPTY), "both were read");
    FLG(publisher_release)(&publisher);
}

static void every_reader_of_an_entry_decodes_it_under_the_same_flag(void) {
    static FLG(t) ring;
    FLG(publisher_t) publisher = {0};
    FLG(reader_t) early;
    FLG(reader_t) late;
    const flagged_element_t first = make_event(5u, true, 500u);
    const flagged_element_t second = make_event(6u, false, 0u);
    CHECK(FLG(init)(&ring) && FLG(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    early = FLG(reader_oldest)(&ring);
    late = FLG(reader_oldest)(&ring);
    (void)publish_flagged(&publisher, &first, true);
    (void)publish_flagged(&publisher, &second, false);
    expect_flagged(&ring, &early, 0u, &first, true);
    expect_flagged(&ring, &early, 1u, &second, false);
    expect_flagged(&ring, &late, 0u, &first, true);
    expect_flagged(&ring, &late, 1u, &second, false);
    FLG(publisher_release)(&publisher);
}

static void a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under(void) {
    static FLG(t) ring;
    FLG(publisher_t) publisher = {0};
    FLG(reader_t) slow;
    FLG(read_result_t) lagged;
    flagged_element_t sent[5];
    flagged_element_t got;
    unsigned n;
    CHECK(FLG(init)(&ring) && FLG(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    slow = FLG(reader_oldest)(&ring);

    /* Five entries into a log that retains three: the window is [2, 5). The flag
     * alternates, so a flag that stayed with the wrong slot would show. */
    for (n = 0; n < 5u; n++) {
        const bool has_value = (n % 2u) == 0u;
        sent[n] = make_event(n + 1u, has_value, 100u + n);
        (void)publish_flagged(&publisher, &sent[n], has_value);
    }
    memset(&got, 0, sizeof got);
    lagged = FLG(read)(&ring, &slow, &got);
    CHECK(lagged.status == FLG_U(READ_LAGGED) && lagged.skipped == 2u, "the reader is told it missed two");
    for (n = 2; n < 5u; n++) {
        expect_flagged(&ring, &slow, n, &sent[n], (n % 2u) == 0u);
    }
    memset(&got, 0, sizeof got);
    lagged = FLG(read)(&ring, &slow, &got);
    CHECK(lagged.status == FLG_U(READ_EMPTY), "nothing is left after the window");
    FLG(publisher_release)(&publisher);
}

static void a_flag_its_element_contradicts_is_refused_by_the_codec_and_nothing_is_published(void) {
    /* The codec checks a flag input against the field it gates before it writes a
     * byte (SCE_FORGE_CODEC_PRESENT_IF_MISMATCH, as the Rust codec does). The log
     * stores what the codec accepted and judges nothing of its own, so a
     * contradiction never reaches an entry: nothing is published and no sequence
     * number is taken. */
    static FLG(t) ring;
    FLG(publisher_t) publisher = {0};
    FLG(reader_t) reader;
    FLG(publish_result_t) published;
    FLG(read_result_t) read;
    flagged_element_t got;
    const FLG(flags_t) clear = flags_of(false);
    const FLG(flags_t) set = flags_of(true);
    const flagged_element_t present = make_event(7u, true, 1u);
    const flagged_element_t absent = make_event(8u, false, 0u);
    CHECK(FLG(init)(&ring) && FLG(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = FLG(reader_oldest)(&ring);

    /* The value is there and the flag says it is not. */
    published = FLG(publish)(&publisher, &present, &clear);
    CHECK(published.status == FLG_U(PUBLISH_ENCODE_FAILED) &&
              published.codec_status == SCE_FORGE_CODEC_PRESENT_IF_MISMATCH,
          "a value under a flag that says it is absent is refused by the codec");
    CHECK(FLG(head)(&ring) == 0u, "nothing was published");

    /* The value is absent and the flag says it is there. */
    published = FLG(publish)(&publisher, &absent, &set);
    CHECK(published.status == FLG_U(PUBLISH_ENCODE_FAILED) &&
              published.codec_status == SCE_FORGE_CODEC_PRESENT_IF_MISMATCH,
          "an absent value under a flag that says it is there is refused by the codec");
    CHECK(FLG(head)(&ring) == 0u, "nothing was published");
    memset(&got, 0, sizeof got);
    read = FLG(read)(&ring, &reader, &got);
    CHECK(read.status == FLG_U(READ_EMPTY), "a reader sees nothing of either");

    /* An element and a flag that agree are published after the refused ones, as
     * entry 0: a refusal does not spend a sequence number. */
    published = FLG(publish)(&publisher, &present, &set);
    CHECK(published.status == FLG_U(PUBLISH_OK) && published.seq == 0u, "a refusal spends no sequence number");
    FLG(publisher_release)(&publisher);
}

/* ---- The tag of a variant that has none of its own ---- */

/* The element whose arm `tag` selects out of `length` bytes, which is how the codec
 * makes one: the wire form carries no tag, so the caller gives it on decode. */
static choice_element_t choice_from(const uint8_t *bytes, size_t length, uint8_t tag) {
    choice_element_t element;
    sce_forge_cursor_t cursor = sce_forge_cursor_init(bytes, length);
    memset(&element, 0, sizeof element);
    CHECK(broadcast_log_conformance_choice_decode(&cursor, &element, tag) == SCE_FORGE_CODEC_OK,
          "the arm bytes decode under tag %u", (unsigned)tag);
    return element;
}

/* The bytes of `element` as its codec writes them; the arm's own bytes only. */
static size_t choice_bytes(const choice_element_t *element, uint8_t *buffer, size_t capacity) {
    size_t length = 0u;
    CHECK(broadcast_log_conformance_choice_encode_to_buf(element, buffer, capacity, &length) == SCE_FORGE_CODEC_OK,
          "the element encodes");
    return length;
}

static uint64_t publish_chosen(CHS(publisher_t) * publisher, const choice_element_t *element, uint8_t tag) {
    const CHS(publish_result_t) result = CHS(publish)(publisher, element, tag);
    CHECK(result.status == CHS_U(PUBLISH_OK), "an element within its bound is published under its tag");
    return result.seq;
}

/* The next read of `reader` is the entry `seq`: an element that encodes to the same
 * `length` bytes as `want`, which for these arms is the same arm, with the tag it was
 * published with. */
static void expect_chosen(const CHS(t) * ring, CHS(reader_t) * reader, uint64_t seq, const uint8_t *want_bytes,
                          size_t want_length, uint8_t tag) {
    choice_element_t got;
    CHS(read_result_t) result;
    uint8_t buffer[CHS_U(BUFFER_BYTES)];
    size_t length;
    memset(&got, 0, sizeof got);
    result = CHS(read)(ring, reader, &got);
    CHECK(result.status == CHS_U(READ_ITEM), "entry %u: an item", (unsigned)seq);
    CHECK(result.seq == seq, "entry %u: it carries sequence number %u", (unsigned)seq, (unsigned)result.seq);
    CHECK(result.tag == tag, "entry %u: the tag it was published with, %u, not %u", (unsigned)seq, (unsigned)tag,
          (unsigned)result.tag);
    memset(buffer, 0, sizeof buffer);
    length = choice_bytes(&got, buffer, sizeof buffer);
    CHECK(length == want_length && memcmp(buffer, want_bytes, want_length) == 0,
          "entry %u: it comes back as the arm it was published as (%u bytes, not %u)", (unsigned)seq, (unsigned)length,
          (unsigned)want_length);
}

static void the_tag_is_in_the_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold(void) {
    static CHS(t) ring;
    CHS(publisher_t) publisher = {0};
    sce_blog_reader_t reader;
    uint64_t words[CHS_U(WORDS)];
    const uint8_t small_bytes[1] = {9u};
    const uint8_t large_bytes[2] = {0x02u, 0x01u};
    const choice_element_t small = choice_from(small_bytes, sizeof small_bytes, 1u);
    const choice_element_t large = choice_from(large_bytes, sizeof large_bytes, 0u);
    CHECK(CHS(init)(&ring) && CHS(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    (void)publish_chosen(&publisher, &small, 1u);
    (void)publish_chosen(&publisher, &large, 0u);

    reader = sce_blog_reader_oldest(&ring.core);
    memset(words, 0xEE, sizeof words);
    read_words(&ring.core, &reader, words);
    CHECK(words[0] == 1u && words[1] == 1u && words[2] == UINT64_C(9),
          "the small arm under tag 1 is [1, 1, 9], not [%u, %u, %u]", (unsigned)words[0], (unsigned)words[1],
          (unsigned)words[2]);
    memset(words, 0xEE, sizeof words);
    read_words(&ring.core, &reader, words);
    CHECK(words[0] == 2u && words[1] == 0u && words[2] == UINT64_C(0x0102),
          "the large arm under tag 0 is [2, 0, 0x0102], not [%u, %u, 0x%X]", (unsigned)words[0], (unsigned)words[1],
          (unsigned)words[2]);
    CHS(publisher_release)(&publisher);
}

static void a_reader_gets_back_the_tag_an_entry_was_published_under(void) {
    static CHS(t) ring;
    CHS(publisher_t) publisher = {0};
    CHS(reader_t) reader;
    CHS(read_result_t) empty;
    choice_element_t got;
    /* The wire form of an arm carries no tag, so the two arms' bytes are the arm's own
     * and which arm a frame is comes from the tag stored beside it. */
    const uint8_t small_nine[1] = {9u};
    const uint8_t large_bytes[2] = {0x02u, 0x01u};
    const uint8_t small_max[1] = {255u};
    const choice_element_t sent_small_nine = choice_from(small_nine, sizeof small_nine, 1u);
    const choice_element_t sent_large = choice_from(large_bytes, sizeof large_bytes, 0u);
    const choice_element_t sent_small_max = choice_from(small_max, sizeof small_max, 1u);
    CHECK(CHS(init)(&ring) && CHS(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = CHS(reader_oldest)(&ring);
    memset(&got, 0, sizeof got);
    empty = CHS(read)(&ring, &reader, &got);
    CHECK(empty.status == CHS_U(READ_EMPTY), "a new log has nothing to read");

    CHECK(publish_chosen(&publisher, &sent_small_nine, 1u) == 0u, "publish 0 takes sequence number 0");
    CHECK(publish_chosen(&publisher, &sent_large, 0u) == 1u, "publish 1 takes sequence number 1");
    CHECK(publish_chosen(&publisher, &sent_small_max, 1u) == 2u, "publish 2 takes sequence number 2");
    expect_chosen(&ring, &reader, 0u, small_nine, sizeof small_nine, 1u);
    expect_chosen(&ring, &reader, 1u, large_bytes, sizeof large_bytes, 0u);
    expect_chosen(&ring, &reader, 2u, small_max, sizeof small_max, 1u);
    memset(&got, 0, sizeof got);
    empty = CHS(read)(&ring, &reader, &got);
    CHECK(empty.status == CHS_U(READ_EMPTY), "all three were read");
    CHS(publisher_release)(&publisher);
}

int main(void) {
    the_headers_state_the_context_words_of_an_entry();
    the_context_is_in_the_word_after_the_length_and_the_bytes_follow_it();
    a_reader_gets_back_the_flag_an_entry_was_published_under();
    every_reader_of_an_entry_decodes_it_under_the_same_flag();
    a_reader_the_publisher_laps_resumes_at_an_entry_with_the_flag_it_was_published_under();
    a_flag_its_element_contradicts_is_refused_by_the_codec_and_nothing_is_published();
    the_tag_is_in_the_context_word_and_the_arms_own_bytes_are_all_the_bytes_hold();
    a_reader_gets_back_the_tag_an_entry_was_published_under();
    (void)printf("generated broadcast logs with caller inputs (C11): %u check(s), %u failure(s)\n", g_checks,
                 g_failures);
    return g_failures == 0u ? 0 : 1;
}
