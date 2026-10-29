/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The broadcast-log kind's generated headers (RFC §synth-5-S, ADR 0062), compiled and used.
 *
 * sce-build's tests read what the generator writes for a log and look for text in
 * it, which shows the text is what the template says and nothing about whether
 * the names in it are the names the runtime and the codecs have, or whether the
 * bytes it packs read back. Here the headers the generator writes for three logs
 * are compiled over the element headers they include and the runtime's real log
 * (sce/forge/broadcast_log.h), and the typed functions are used on one thread: a
 * rename in the runtime or a codec, or a constant the template computes wrongly,
 * stops the build or fails an assertion. The runtime's own properties, under real
 * threads, are broadcast_log_runtime_test.c's; this file holds what a generated
 * header adds.
 *
 * The three logs, each generated from a document set by `sce-codegen orchestrate`
 * (a slot is sized from the element document's largest encoded size, which only a
 * set can read):
 *
 *   broadcast_log_conformance_literal   retention 3 written as a constant, over an
 *                                       element of exactly three bytes
 *   broadcast_log_conformance_deployed  retention a deploy limit (5), over a record
 *                                       of a tag and a payload of at most eight bytes
 *   broadcast_log_c_cbor                retention 3, over a CBOR map with a text and
 *                                       byte strings: the C codec's struct owns them
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "broadcast_log_c_cbor.h"
#include "broadcast_log_conformance_deployed.h"
#include "broadcast_log_conformance_literal.h"

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
#define LIT(NAME) broadcast_log_conformance_literal_##NAME
#define LIT_U(NAME) BROADCAST_LOG_CONFORMANCE_LITERAL_##NAME
#define DEP(NAME) broadcast_log_conformance_deployed_##NAME
#define DEP_U(NAME) BROADCAST_LOG_CONFORMANCE_DEPLOYED_##NAME
#define CBR(NAME) broadcast_log_c_cbor_##NAME
#define CBR_U(NAME) BROADCAST_LOG_C_CBOR_##NAME

/* ---- The literal log: retention 3, an element of three bytes ---- */

static broadcast_log_conformance_event_t make_event(unsigned sensor, unsigned value) {
    broadcast_log_conformance_event_t e;
    memset(&e, 0, sizeof e);
    e.sensor_id = (uint8_t)sensor;
    e.value = (uint16_t)value;
    return e;
}

static uint64_t publish_event(LIT(publisher_t) * publisher, unsigned sensor, unsigned value) {
    const broadcast_log_conformance_event_t e = make_event(sensor, value);
    const LIT(publish_result_t) result = LIT(publish)(publisher, &e);
    CHECK(result.status == LIT_U(PUBLISH_OK), "an element within its bound is published");
    return result.seq;
}

/* The next read of `reader` is the entry `seq`, whole. */
static void expect_item(const LIT(t) * ring, LIT(reader_t) * reader, uint64_t seq, unsigned sensor, unsigned value) {
    broadcast_log_conformance_event_t got;
    LIT(read_result_t) result;
    memset(&got, 0, sizeof got);
    result = LIT(read)(ring, reader, &got);
    CHECK(result.status == LIT_U(READ_ITEM), "entry %u: an item", (unsigned)seq);
    CHECK(result.seq == seq, "entry %u: it carries sequence number %u", (unsigned)seq, (unsigned)result.seq);
    CHECK(got.sensor_id == sensor && got.value == value, "entry %u: whole, sensor %u and value %u", (unsigned)seq,
          (unsigned)got.sensor_id, (unsigned)got.value);
}

/* The next read of `reader` answers `status` and leaves `*out` as it was. */
static LIT(read_result_t) expect_no_entry(const LIT(t) * ring, LIT(reader_t) * reader, LIT(read_status_t) status) {
    broadcast_log_conformance_event_t got;
    LIT(read_result_t) result;
    memset(&got, 0xAB, sizeof got);
    result = LIT(read)(ring, reader, &got);
    CHECK(result.status == status, "the answer is %d, not %d", (int)status, (int)result.status);
    /* A frame the codec refuses holds no value to read (its contract), so only the
     * answers that never reach the codec promise the element is as it was. */
    if (status != LIT_U(READ_DECODE_FAILED)) {
        CHECK(got.sensor_id == 0xABu && got.value == 0xABABu, "nothing was decoded into the caller's element");
    }
    return result;
}

static void the_header_states_what_the_document_and_its_element_give(void) {
    /* Retention 3 is the document's; slots are 3 + 1 = 4. The element is a uint8
     * and a uint16, 1 + 2 = 3 bytes, so an entry is one length word and
     * ceil(3 / 8) = 1 word of bytes: 1 + 1 = 2 words. */
    CHECK(LIT_U(RETENTION) == 3u, "the retention is the document's: three");
    CHECK(LIT_U(SLOTS) == 4u, "slots are the retention and one more");
    CHECK(LIT_U(MAX_ENCODED_BYTES) == 3u, "the element encodes to at most 1 + 2 = 3 bytes");
    CHECK(LIT_U(MAX_ENCODED_BYTES) == BROADCAST_LOG_CONFORMANCE_EVENT_MAX_BYTES,
          "the log's bound is the codec's own for a codec of fixed fields");
    CHECK(LIT_U(WORDS) == 2u, "an entry is a length word and one word of bytes: 1 + ceil(3 / 8) = 2");
    CHECK(strcmp(LIT_U(DECLARED_PROGRESS), "wait-free") == 0, "the document declared wait-free");
    /* The storage is the slots, each a stamp and its words: 4 * (2 + 1) = 12. */
    CHECK(sizeof(((LIT(t) *)0)->storage) / sizeof(uint64_t) == 12u, "four slots of a stamp and two words");
}

static void entries_come_back_whole_and_in_order(void) {
    static LIT(t) ring;
    LIT(publisher_t) publisher = {0};
    LIT(reader_t) reader;
    unsigned i;
    CHECK(LIT(init)(&ring), "the shape the document states is refused");
    CHECK(LIT(publisher_acquire)(&ring, &publisher), "a new log hands out the publisher's place");
    reader = LIT(reader_oldest)(&ring);
    CHECK(LIT(head)(&ring) == 0u, "a new log has published nothing");
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_EMPTY));

    /* Three entries fit the retention of three: sequence numbers 0, 1 and 2. */
    for (i = 0; i < 3u; i++) {
        CHECK(publish_event(&publisher, 10u + i, 1000u + i) == i, "publish %u takes sequence number %u", i, i);
    }
    CHECK(LIT(head)(&ring) == 3u, "three publishes: the head is three");
    for (i = 0; i < 3u; i++) {
        expect_item(&ring, &reader, i, 10u + i, 1000u + i);
    }
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_EMPTY));
    CHECK(reader.cursor == 3u && reader.lost == 0u, "the reader holds the next entry and lost none");
    LIT(publisher_release)(&publisher);
}

static void a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest(void) {
    static LIT(t) ring;
    LIT(publisher_t) publisher = {0};
    LIT(reader_t) reader;
    LIT(read_result_t) lagged;
    unsigned i;
    CHECK(LIT(init)(&ring) && LIT(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = LIT(reader_oldest)(&ring);

    /* Five entries, 0 to 4, are published before the reader reads. The log retains
     * the last three, so the window is [5 - 3, 5) = [2, 5), and the reader at 0 is
     * outside it: it has missed (5 - 3) - 0 = 2 entries. */
    for (i = 0; i < 5u; i++) {
        (void)publish_event(&publisher, 20u + i, 2000u + i);
    }
    lagged = expect_no_entry(&ring, &reader, LIT_U(READ_LAGGED));
    CHECK(lagged.skipped == 2u, "the reader is told it missed two: %u", (unsigned)lagged.skipped);
    CHECK(reader.cursor == 2u && reader.lost == 2u, "it resumes at the oldest retained and carries the loss");
    for (i = 2; i < 5u; i++) {
        expect_item(&ring, &reader, i, 20u + i, 2000u + i);
    }
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_EMPTY));
    CHECK(reader.lost == 2u, "reading what is retained loses nothing more");
    LIT(publisher_release)(&publisher);
}

static void two_readers_keep_cursors_of_their_own(void) {
    static LIT(t) ring;
    LIT(publisher_t) publisher = {0};
    LIT(reader_t) at_the_oldest;
    LIT(reader_t) at_the_head;
    CHECK(LIT(init)(&ring) && LIT(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    (void)publish_event(&publisher, 30u, 3000u);
    (void)publish_event(&publisher, 31u, 3001u);
    at_the_oldest = LIT(reader_oldest)(&ring);
    at_the_head = LIT(reader_head)(&ring);
    CHECK(at_the_oldest.cursor == 0u && at_the_head.cursor == 2u, "the readers open at the oldest and at the head");

    /* The reader at the oldest reads entry 0; the one at the head sees nothing yet
     * and its cursor is untouched by the other's read. */
    expect_item(&ring, &at_the_oldest, 0u, 30u, 3000u);
    (void)expect_no_entry(&ring, &at_the_head, LIT_U(READ_EMPTY));
    CHECK(at_the_oldest.cursor == 1u && at_the_head.cursor == 2u, "each reader moved only itself");

    (void)publish_event(&publisher, 32u, 3002u);
    expect_item(&ring, &at_the_oldest, 1u, 31u, 3001u);
    expect_item(&ring, &at_the_head, 2u, 32u, 3002u);
    expect_item(&ring, &at_the_oldest, 2u, 32u, 3002u);
    CHECK(at_the_oldest.cursor == 3u && at_the_head.cursor == 3u, "both are at the head");
    CHECK(at_the_oldest.lost == 0u && at_the_head.lost == 0u, "neither lost anything");

    /* A reader opened at a sequence number past the head is held at the head. */
    {
        const LIT(reader_t) past = LIT(reader_at)(&ring, 99u);
        CHECK(past.cursor == 3u, "a reader asked for entry 99 of a log of three holds the head");
    }
    LIT(publisher_release)(&publisher);
}

static void the_publishers_place_is_taken_once_and_its_successor_continues_the_sequence(void) {
    static LIT(t) ring;
    LIT(publisher_t) first = {0};
    LIT(publisher_t) second = {0};
    CHECK(LIT(init)(&ring), "a log");
    CHECK(LIT(publisher_acquire)(&ring, &first), "the publisher's place is free");
    CHECK(!LIT(publisher_acquire)(&ring, &second), "a log written by two publishers is not this algorithm");
    CHECK(publish_event(&first, 1u, 1u) == 0u && publish_event(&first, 2u, 2u) == 1u, "two entries: 0 and 1");
    LIT(publisher_release)(&first);
    CHECK(LIT(publisher_acquire)(&ring, &second), "the place is free again once it is given back");
    CHECK(publish_event(&second, 3u, 3u) == 2u, "the successor continues the sequence where the last stopped");
    LIT(publisher_release)(&second);
}

static void an_element_is_published_only_while_the_log_can_number_another(void) {
    static LIT(t) ring;
    LIT(publisher_t) publisher = {0};
    const broadcast_log_conformance_event_t e = make_event(1u, 1u);
    LIT(publish_result_t) result;
    CHECK(LIT(init)(&ring) && LIT(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    (void)publish_event(&publisher, 1u, 1u);
    publisher.next = SCE_BLOG_HORIZON;
    result = LIT(publish)(&publisher, &e);
    CHECK(result.status == LIT_U(PUBLISH_HORIZON), "a log at its horizon publishes nothing more");
    CHECK(LIT(head)(&ring) == 1u, "and its head did not move");
    LIT(publisher_release)(&publisher);
}

/* What the log reads back is only what the typed publish wrote; the runtime's own
 * publish writes any words, and a read refuses what no publish of this log wrote. */
static void a_stored_length_above_the_bound_is_refused_and_never_copied(void) {
    static LIT(t) ring;
    LIT(publisher_t) publisher = {0};
    LIT(reader_t) reader;
    LIT(read_result_t) result;
    uint64_t entry[LIT_U(WORDS)];
    CHECK(LIT(init)(&ring) && LIT(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = LIT(reader_head)(&ring);

    /* A length of 4 is one past the bound of 3. */
    memset(entry, 0, sizeof entry);
    entry[0] = 4u;
    (void)sce_blog_publish(&publisher, entry);
    result = expect_no_entry(&ring, &reader, LIT_U(READ_BAD_LENGTH));
    CHECK(result.seq == 0u, "it is the entry 0 that is refused");
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_EMPTY));

    /* The largest length a word holds is not an index either. */
    entry[0] = UINT64_MAX;
    (void)sce_blog_publish(&publisher, entry);
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_BAD_LENGTH));
    (void)expect_no_entry(&ring, &reader, LIT_U(READ_EMPTY));

    /* A length of exactly the bound, three bytes 0x0A 0xE8 0x03, is the element
     * sensor 10 and value 0x03E8 = 1000, little-endian in the word:
     * 0x0A + 0xE8 * 2^8 + 0x03 * 2^16 = 10 + 59392 + 196608 = 256010. */
    entry[0] = 3u;
    entry[1] = 256010u;
    (void)sce_blog_publish(&publisher, entry);
    expect_item(&ring, &reader, 2u, 10u, 1000u);

    /* One byte stored is within the bound and is not an element: the codec says
     * so, and nothing is handed back. */
    entry[0] = 1u;
    entry[1] = 0x0Au;
    (void)sce_blog_publish(&publisher, entry);
    result = expect_no_entry(&ring, &reader, LIT_U(READ_DECODE_FAILED));
    CHECK(result.codec_status == SCE_FORGE_CODEC_NEED_MORE_BYTES, "the codec wants two more bytes");
    LIT(publisher_release)(&publisher);
}

/* ---- The deployed log: retention a deploy limit, over a record with a payload ---- */

static void the_deployed_header_states_the_limit_the_deploy_gave_and_the_records_largest_encoding(void) {
    /* The deploy gives event_retention 5; slots are 5 + 1 = 6. The record is a
     * one-byte tag and a payload of at most eight bytes: 1 + 8 = 9 bytes, so an
     * entry is a length word and ceil(9 / 8) = 2 words of bytes: 1 + 2 = 3. */
    CHECK(DEP_U(RETENTION) == 5u, "the retention is the deploy's limit: five");
    CHECK(DEP_U(SLOTS) == 6u, "slots are the retention and one more");
    CHECK(DEP_U(MAX_ENCODED_BYTES) == 9u, "the record encodes to at most 1 + 8 = 9 bytes");
    CHECK(DEP_U(WORDS) == 3u, "a length word and two words of bytes: 1 + ceil(9 / 8) = 3");
    CHECK(sizeof(((DEP(t) *)0)->storage) / sizeof(uint64_t) == 24u, "six slots of a stamp and three words: 6 * 4 = 24");
}

static void a_record_whose_bytes_cross_a_word_comes_back_whole(void) {
    static DEP(t) ring;
    DEP(publisher_t) publisher = {0};
    DEP(reader_t) reader;
    broadcast_log_conformance_record_t sent;
    broadcast_log_conformance_record_t got;
    DEP(publish_result_t) published;
    DEP(read_result_t) read;
    unsigned lengths[3] = {8u, 3u, 0u};
    unsigned n;
    unsigned i;
    CHECK(DEP(init)(&ring) && DEP(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = DEP(reader_oldest)(&ring);

    /* The three records encode to 1 + 8 = 9, 1 + 3 = 4 and 1 + 0 = 1 bytes: the
     * first has its ninth byte in the second word of bytes. */
    for (n = 0; n < 3u; n++) {
        memset(&sent, 0, sizeof sent);
        sent.tag = (uint8_t)(0xA0u + n);
        for (i = 0; i < lengths[n]; i++) {
            sent.payload[i] = (uint8_t)(i + 1u);
        }
        sent.payload_len = lengths[n];
        published = DEP(publish)(&publisher, &sent);
        CHECK(published.status == DEP_U(PUBLISH_OK) && published.seq == n, "record %u is published as entry %u", n, n);
    }
    for (n = 0; n < 3u; n++) {
        memset(&got, 0xEE, sizeof got);
        read = DEP(read)(&ring, &reader, &got);
        CHECK(read.status == DEP_U(READ_ITEM) && read.seq == n, "record %u: an item", n);
        CHECK(got.tag == 0xA0u + n, "record %u: its tag", n);
        CHECK(got.payload_len == lengths[n], "record %u: its payload is %u bytes, not %u", n, lengths[n],
              (unsigned)got.payload_len);
        for (i = 0; i < lengths[n]; i++) {
            CHECK(got.payload[i] == i + 1u, "record %u: payload byte %u", n, i);
        }
    }
    read = DEP(read)(&ring, &reader, &got);
    CHECK(read.status == DEP_U(READ_EMPTY), "all three were read");
    DEP(publisher_release)(&publisher);
}

/* ---- The CBOR log: the C codec's struct owns the text and the byte strings ---- */

static void the_cbor_header_is_sized_from_the_codecs_own_bound(void) {
    /* The log's number is the one the CBOR codec's header states for the same map:
     * the two are one computation. An entry is a length word and as many words of
     * bytes as the bound takes at eight to a word. */
    CHECK(CBR_U(MAX_ENCODED_BYTES) == CODEC_CBOR_MAP_MAX_BYTES, "the log and the codec state one bound");
    CHECK(CBR_U(WORDS) == 1u + (CODEC_CBOR_MAP_MAX_BYTES + 7u) / 8u, "a length word and the bound's words of bytes");
    CHECK(CBR_U(RETENTION) == 3u && CBR_U(SLOTS) == 4u, "the document's retention of three, and one more slot");
}

static void a_cbor_element_with_text_and_bytes_comes_back_whole_and_owns_them(void) {
    static CBR(t) ring;
    static codec_cbor_map_t sent;
    static codec_cbor_map_t got;
    CBR(publisher_t) publisher = {0};
    CBR(reader_t) reader;
    CBR(publish_result_t) published;
    CBR(read_result_t) read;
    unsigned i;
    CHECK(CBR(init)(&ring) && CBR(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = CBR(reader_oldest)(&ring);

    memset(&sent, 0, sizeof sent);
    memcpy(sent.source, "sensor", 6);
    sent.source_len = 6u;
    memset(sent.id, 0x22, sizeof sent.id);
    sent.id_len = sizeof sent.id;
    sent.pattern = 7u;
    for (i = 0; i < 100u; i++) {
        sent.payload[i] = (uint8_t)i;
    }
    sent.payload_len = 100u;
    sent.deadline = sce_optional_uint64_some(UINT64_C(1700000000000));
    sent.urgent.present = true;
    sent.urgent.value = true;

    published = CBR(publish)(&publisher, &sent);
    CHECK(published.status == CBR_U(PUBLISH_OK) && published.seq == 0u, "the element is published as entry 0");
    memset(&got, 0, sizeof got);
    read = CBR(read)(&ring, &reader, &got);
    CHECK(read.status == CBR_U(READ_ITEM) && read.seq == 0u, "and read back as an item");
    CHECK(got.source_len == 6u && memcmp(got.source, "sensor", 6) == 0, "its text");
    CHECK(got.id_len == 16u && memcmp(got.id, sent.id, 16) == 0, "its exact-length byte string");
    CHECK(got.pattern == 7u, "its number");
    CHECK(got.payload_len == 100u && memcmp(got.payload, sent.payload, 100) == 0, "its bounded byte string");
    CHECK(got.deadline.present && got.deadline.value == UINT64_C(1700000000000), "an optional entry that is present");
    CHECK(got.urgent.present && got.urgent.value, "another that is present");

    /* The decoded struct holds copies: the log's slot and the buffer the entry was
     * unpacked into are free to be overwritten, and the element stays what it was. */
    for (i = 0; i < 4u; i++) {
        published = CBR(publish)(&publisher, &sent);
        CHECK(published.status == CBR_U(PUBLISH_OK), "later entries overwrite the slot of the first");
    }
    CHECK(got.source_len == 6u && memcmp(got.source, "sensor", 6) == 0, "the text was copied out of the log");
    CHECK(got.payload_len == 100u && memcmp(got.payload, sent.payload, 100) == 0, "so was the byte string");
    CBR(publisher_release)(&publisher);
}

static void an_element_the_codec_refuses_is_not_published_and_not_truncated(void) {
    static CBR(t) ring;
    static codec_cbor_map_t sent;
    CBR(publisher_t) publisher = {0};
    CBR(publish_result_t) published;
    CBR(reader_t) reader;
    broadcast_log_c_cbor_read_result_t read;
    static codec_cbor_map_t got;
    CHECK(CBR(init)(&ring) && CBR(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = CBR(reader_oldest)(&ring);

    memset(&sent, 0, sizeof sent);
    memset(sent.source, 's', sizeof sent.source);
    memset(sent.id, 0x22, sizeof sent.id);
    sent.id_len = sizeof sent.id;
    sent.payload_len = 1u;

    /* A text of 65 bytes where the bound is 64: the codec refuses it, and the log
     * neither stores the first 64 nor moves its head. */
    sent.source_len = sizeof sent.source + 1u;
    published = CBR(publish)(&publisher, &sent);
    CHECK(published.status == CBR_U(PUBLISH_ENCODE_FAILED), "an element past its bound is a failed publish");
    CHECK(published.codec_status == SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE, "and the codec says why: %d",
          (int)published.codec_status);
    CHECK(CBR(head)(&ring) == 0u, "nothing was published");

    /* An exact-length byte string of 15 bytes where the document says 16. */
    sent.source_len = 6u;
    sent.id_len = 15u;
    published = CBR(publish)(&publisher, &sent);
    CHECK(published.status == CBR_U(PUBLISH_ENCODE_FAILED) &&
              published.codec_status == SCE_FORGE_CODEC_CBOR_WRONG_LENGTH,
          "a byte string of the wrong length is refused by the codec");
    CHECK(CBR(head)(&ring) == 0u, "nothing was published");

    memset(&got, 0, sizeof got);
    read = CBR(read)(&ring, &reader, &got);
    CHECK(read.status == CBR_U(READ_EMPTY), "a reader sees nothing of either");

    /* The element that is within its bounds is published after the refused ones, as
     * entry 0: a refusal does not spend a sequence number. */
    sent.id_len = sizeof sent.id;
    published = CBR(publish)(&publisher, &sent);
    CHECK(published.status == CBR_U(PUBLISH_OK) && published.seq == 0u, "a refusal spends no sequence number");
    CBR(publisher_release)(&publisher);
}

int main(void) {
    the_header_states_what_the_document_and_its_element_give();
    entries_come_back_whole_and_in_order();
    a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest();
    two_readers_keep_cursors_of_their_own();
    the_publishers_place_is_taken_once_and_its_successor_continues_the_sequence();
    an_element_is_published_only_while_the_log_can_number_another();
    a_stored_length_above_the_bound_is_refused_and_never_copied();
    the_deployed_header_states_the_limit_the_deploy_gave_and_the_records_largest_encoding();
    a_record_whose_bytes_cross_a_word_comes_back_whole();
    the_cbor_header_is_sized_from_the_codecs_own_bound();
    a_cbor_element_with_text_and_bytes_comes_back_whole_and_owns_them();
    an_element_the_codec_refuses_is_not_published_and_not_truncated();
    (void)printf("generated broadcast logs (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
