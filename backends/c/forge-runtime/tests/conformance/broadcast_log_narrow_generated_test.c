/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The broadcast-log kind's generated headers for the narrow row (ADR 0062, "Stamp
 * width"), compiled and used.
 *
 * sce-build's tests read what the generator writes for a narrow log and look for text
 * in it, which shows the text is what the template says and nothing about whether the
 * names in it are the names the narrow runtime and the codecs have, or whether the
 * 32-bit words it packs read back. Here the headers the generator writes for two logs
 * are compiled over the element headers they include and the runtime's real narrow log
 * (sce/forge/broadcast_log_narrow.h, at the 32-bit stamp a row keeps), and the typed
 * functions are used on one thread. The runtime's own properties are
 * broadcast_log_narrow_runtime_test.c's and broadcast_log_narrow_runtime_32_test.c's;
 * this file holds what a generated header adds.
 *
 * The two logs, each generated from a document set by `sce-codegen orchestrate` for
 * the machine of a deploy that names the build target and states the longest pause the
 * design relies on (the generator chooses the row from the target):
 *
 *   broadcast_log_c_narrow     a target with a 32-bit compare-and-swap (thumbv7em),
 *                              retention 3 over a record of a tag and a payload of at
 *                              most eight bytes: 9 bytes, which cross the 4-byte words
 *   broadcast_log_c_ownership  a target with loads and stores only (thumbv6m),
 *                              retention 3 over an element of exactly three bytes; its
 *                              header has no publisher taken by a compare-and-swap
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "broadcast_log_c_narrow.h"
#include "broadcast_log_c_ownership.h"

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

/* A name of the generated log of each document: its lower-case identifiers and its
 * upper-case macros. */
#define NRW(NAME) broadcast_log_c_narrow_##NAME
#define NRW_U(NAME) BROADCAST_LOG_C_NARROW_##NAME
#define OWN(NAME) broadcast_log_c_ownership_##NAME
#define OWN_U(NAME) BROADCAST_LOG_C_OWNERSHIP_##NAME

/* ---- The narrow log: a record whose bytes cross a 32-bit word ---- */

static broadcast_log_conformance_record_t make_record(unsigned tag, unsigned length) {
    broadcast_log_conformance_record_t record;
    unsigned i;
    memset(&record, 0, sizeof record);
    record.tag = (uint8_t)tag;
    for (i = 0; i < length; i++) {
        record.payload[i] = (uint8_t)(i + 1u);
    }
    record.payload_len = length;
    return record;
}

static uint64_t publish_record(NRW(publisher_t) * publisher, unsigned tag, unsigned length) {
    const broadcast_log_conformance_record_t record = make_record(tag, length);
    const NRW(publish_result_t) result = NRW(publish)(publisher, &record);
    CHECK(result.status == NRW_U(PUBLISH_OK), "a record within its bound is published");
    return result.seq;
}

/* The next read of `reader` is the entry `seq`, a record of `tag` and `length` payload
 * bytes whole. */
static void expect_record(const NRW(t) * ring, NRW(reader_t) * reader, uint64_t seq, unsigned tag, unsigned length) {
    broadcast_log_conformance_record_t got;
    NRW(read_result_t) result;
    unsigned i;
    memset(&got, 0xEE, sizeof got);
    result = NRW(read)(ring, reader, &got);
    CHECK(result.status == NRW_U(READ_ITEM), "entry %u: an item", (unsigned)seq);
    CHECK(result.seq == seq, "entry %u: it carries sequence number %u", (unsigned)seq, (unsigned)result.seq);
    CHECK(got.tag == tag, "entry %u: its tag", (unsigned)seq);
    CHECK(got.payload_len == length, "entry %u: its payload is %u bytes, not %u", (unsigned)seq, length,
          (unsigned)got.payload_len);
    for (i = 0; i < length; i++) {
        CHECK(got.payload[i] == i + 1u, "entry %u: payload byte %u", (unsigned)seq, i);
    }
}

/* The next read of `reader` answers `status` and decodes nothing. */
static NRW(read_result_t) expect_no_record(const NRW(t) * ring, NRW(reader_t) * reader, NRW(read_status_t) status) {
    broadcast_log_conformance_record_t got;
    NRW(read_result_t) result;
    memset(&got, 0xAB, sizeof got);
    result = NRW(read)(ring, reader, &got);
    CHECK(result.status == status, "the answer is %d, not %d", (int)status, (int)result.status);
    return result;
}

static void the_narrow_header_states_what_the_document_its_element_and_the_target_give(void) {
    /* Retention 3 is the document's; slots are 3 + 1 = 4, a power of two. The record is a
     * one-byte tag and a payload of at most eight bytes: 1 + 8 = 9 bytes, so an entry is a
     * length word and ceil(9 / 4) = 3 words of bytes: 1 + 3 = 4 words of 32 bits. The
     * stamp is 32 bits, which covers a pause of 2^31 less the 4 slots. */
    CHECK(NRW_U(RETENTION) == 3u, "the retention is the document's: three");
    CHECK(NRW_U(SLOTS) == 4u, "slots are the retention and one more");
    CHECK(NRW_U(MAX_ENCODED_BYTES) == 9u, "the record encodes to at most 1 + 8 = 9 bytes");
    CHECK(NRW_U(WORDS) == 4u, "a length word and three words of bytes: 1 + ceil(9 / 4) = 4");
    CHECK(NRW_U(STAMP_BITS) == 32u, "the log was judged against a stamp of 32 bits");
    CHECK(NRW_U(STAMP_BITS) == SCE_BLOGN_STAMP_BITS, "and the runtime keeps it");
    CHECK(NRW_U(WRAP_BOUND_OPS) == UINT64_C(2147483644), "2^31 - 4 publishes between two reads of one reader");
    CHECK(strcmp(NRW_U(DECLARED_PROGRESS), "wait-free") == 0, "the document declared wait-free");
    /* The storage is the slots, each a stamp and its words: 4 * (4 + 1) = 20 words of 32 bits. */
    CHECK(sizeof(((NRW(t) *)0)->storage) / sizeof(uint32_t) == 20u, "four slots of a stamp and four words");
    CHECK(sizeof(((NRW(t) *)0)->storage[0]) == 4u, "the storage is made of 32-bit words");
}

static void records_come_back_whole_and_are_packed_four_bytes_to_a_word(void) {
    static NRW(t) ring;
    NRW(publisher_t) publisher = {0};
    NRW(reader_t) reader;
    const unsigned lengths[3] = {8u, 3u, 0u};
    unsigned n;
    CHECK(NRW(init)(&ring), "the shape the document states is refused");
    CHECK(NRW(publisher_acquire)(&ring, &publisher), "a new log hands out the publisher's place");
    reader = NRW(reader_oldest)(&ring);
    CHECK(NRW(head)(&ring) == 0u, "a new log has published nothing");
    (void)expect_no_record(&ring, &reader, NRW_U(READ_EMPTY));

    /* The three records encode to 1 + 8 = 9, 1 + 3 = 4 and 1 + 0 = 1 bytes. */
    for (n = 0; n < 3u; n++) {
        CHECK(publish_record(&publisher, 0xA0u + n, lengths[n]) == n, "record %u takes sequence number %u", n, n);
    }
    CHECK(NRW(head)(&ring) == 3u, "three publishes: the head is three");

    /* The words the runtime holds for entry 0 are the documented ones: the length 9, then
     * the bytes A0 01 02 03 | 04 05 06 07 | 08 little-endian, four to a word. */
    {
        sce_blogn_reader_t raw = sce_blogn_reader_oldest(&ring.core);
        uint32_t words[NRW_U(WORDS)];
        const sce_blogn_read_t found = sce_blogn_read(&ring.core, &raw, words);
        CHECK(found.status == SCE_BLOGN_ITEM && found.seq == 0u, "the runtime reads entry 0");
        CHECK(words[0] == 9u, "the length word is 9");
        CHECK(words[1] == UINT32_C(0x030201A0), "the first word of bytes: A0 01 02 03");
        CHECK(words[2] == UINT32_C(0x07060504), "the second word of bytes: 04 05 06 07");
        CHECK(words[3] == UINT32_C(0x00000008), "the third word holds the ninth byte alone");
    }

    for (n = 0; n < 3u; n++) {
        expect_record(&ring, &reader, n, 0xA0u + n, lengths[n]);
    }
    (void)expect_no_record(&ring, &reader, NRW_U(READ_EMPTY));
    CHECK(reader.cursor == 3u && reader.lost == 0u, "the reader holds the next entry and lost none");
    NRW(publisher_release)(&publisher);
}

static void a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest(void) {
    static NRW(t) ring;
    NRW(publisher_t) publisher = {0};
    NRW(reader_t) reader;
    NRW(read_result_t) lagged;
    unsigned n;
    CHECK(NRW(init)(&ring) && NRW(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    reader = NRW(reader_oldest)(&ring);

    /* Six entries, 0 to 5, are published before the reader reads. The log retains the
     * last three, so the window is [6 - 3, 6) = [3, 6), and the reader at 0 is outside it:
     * it has missed (6 - 3) - 0 = 3 entries. */
    for (n = 0; n < 6u; n++) {
        (void)publish_record(&publisher, 0x10u + n, 0u);
    }
    lagged = expect_no_record(&ring, &reader, NRW_U(READ_LAGGED));
    CHECK(lagged.skipped == 3u, "the reader is told it missed three: %u", (unsigned)lagged.skipped);
    CHECK(reader.cursor == 3u && reader.lost == 3u, "it resumes at the oldest retained and carries the loss");
    for (n = 3; n < 6u; n++) {
        expect_record(&ring, &reader, n, 0x10u + n, 0u);
    }
    (void)expect_no_record(&ring, &reader, NRW_U(READ_EMPTY));
    CHECK(reader.lost == 3u, "reading what is retained loses nothing more");
    NRW(publisher_release)(&publisher);
}

/* The head, a reader's cursor and a sequence number are residues modulo 2^32, and a
 * reader opened at one the log has not reached is held at the head. */
static void a_position_is_a_residue_and_one_past_the_head_is_the_head(void) {
    static NRW(t) ring;
    NRW(publisher_t) publisher = {0};
    NRW(reader_t) reader;
    CHECK(NRW(init)(&ring) && NRW(publisher_acquire)(&ring, &publisher), "a log and its publisher");
    (void)publish_record(&publisher, 1u, 0u);
    (void)publish_record(&publisher, 2u, 0u);
    reader = NRW(reader_at)(&ring, 1u);
    CHECK(reader.cursor == 1u, "a reader asked for entry 1, which is 1 behind the head, holds it");
    reader = NRW(reader_at)(&ring, 99u);
    CHECK(reader.cursor == 2u, "a reader asked for entry 99 of a log of two holds the head");
    reader = NRW(reader_head)(&ring);
    CHECK(reader.cursor == 2u, "a reader at the head holds it");
    NRW(publisher_release)(&publisher);
}

static void the_publishers_place_is_one_whichever_form_takes_it(void) {
    static NRW(t) ring;
    NRW(publisher_t) first = {0};
    NRW(publisher_t) second = {0};
    CHECK(NRW(init)(&ring), "a log");
    CHECK(NRW(publisher_acquire)(&ring, &first), "the publisher's place is free");
    CHECK(!NRW(publisher_acquire)(&ring, &second), "a log written by two publishers is not this algorithm");
    CHECK(!NRW(split)(&ring, &second), "and the ownership form refuses the place the swap holds");
    CHECK(publish_record(&first, 1u, 0u) == 0u && publish_record(&first, 2u, 0u) == 1u, "two entries: 0 and 1");
    NRW(publisher_release)(&first);
    CHECK(NRW(split)(&ring, &second), "the ownership form takes the place once it is given back");
    CHECK(!NRW(publisher_acquire)(&ring, &first), "which the swap then finds taken");
    CHECK(publish_record(&second, 3u, 0u) == 2u, "the successor continues the sequence where the last stopped");
    NRW(publisher_release)(&second);
    first = NRW(publisher_unchecked)(&ring);
    CHECK(publish_record(&first, 4u, 0u) == 3u, "the publisher taken on the caller's word continues it too");
    CHECK(!NRW(split)(&ring, &second), "and holds the place against the others");
    NRW(publisher_release)(&first);
}

/* ---- The ownership log: a target with loads and stores only ---- */

static broadcast_log_conformance_event_t make_event(unsigned sensor, unsigned value) {
    broadcast_log_conformance_event_t event;
    memset(&event, 0, sizeof event);
    event.sensor_id = (uint8_t)sensor;
    event.value = (uint16_t)value;
    return event;
}

static uint64_t publish_event(OWN(publisher_t) * publisher, unsigned sensor, unsigned value) {
    const broadcast_log_conformance_event_t event = make_event(sensor, value);
    const OWN(publish_result_t) result = OWN(publish)(publisher, &event);
    CHECK(result.status == OWN_U(PUBLISH_OK), "an element within its bound is published");
    return result.seq;
}

static void expect_event(const OWN(t) * ring, OWN(reader_t) * reader, uint64_t seq, unsigned sensor, unsigned value) {
    broadcast_log_conformance_event_t got;
    OWN(read_result_t) result;
    memset(&got, 0, sizeof got);
    result = OWN(read)(ring, reader, &got);
    CHECK(result.status == OWN_U(READ_ITEM), "entry %u: an item", (unsigned)seq);
    CHECK(result.seq == seq, "entry %u: it carries sequence number %u", (unsigned)seq, (unsigned)result.seq);
    CHECK(got.sensor_id == sensor && got.value == value, "entry %u: whole, sensor %u and value %u", (unsigned)seq,
          (unsigned)got.sensor_id, (unsigned)got.value);
}

static void the_ownership_header_states_the_same_row_for_a_target_with_no_swap(void) {
    /* An element of a uint8 and a uint16: 1 + 2 = 3 bytes, so an entry is a length word and
     * ceil(3 / 4) = 1 word of bytes: 1 + 1 = 2 words. */
    CHECK(OWN_U(RETENTION) == 3u && OWN_U(SLOTS) == 4u, "retention 3 keeps 4 slots");
    CHECK(OWN_U(MAX_ENCODED_BYTES) == 3u, "the element encodes to at most 1 + 2 = 3 bytes");
    CHECK(OWN_U(WORDS) == 2u, "a length word and one word of bytes: 1 + ceil(3 / 4) = 2");
    CHECK(OWN_U(STAMP_BITS) == 32u, "a stamp of 32 bits");
    CHECK(sizeof(((OWN(t) *)0)->storage) / sizeof(uint32_t) == 12u, "four slots of a stamp and two words");
}

static void entries_come_back_through_the_ownership_forms(void) {
    static OWN(t) ring;
    OWN(publisher_t) publisher = {0};
    OWN(reader_t) reader;
    unsigned i;
    CHECK(OWN(init)(&ring), "the shape the document states is refused");
    CHECK(OWN(split)(&ring, &publisher), "a log the caller holds exclusively hands out its place");
    reader = OWN(reader_oldest)(&ring);
    for (i = 0; i < 3u; i++) {
        CHECK(publish_event(&publisher, 10u + i, 1000u + i) == i, "publish %u takes sequence number %u", i, i);
    }
    for (i = 0; i < 3u; i++) {
        expect_event(&ring, &reader, i, 10u + i, 1000u + i);
    }
    OWN(publisher_release)(&publisher);

    /* A log in static storage is shared from the start: its successor is taken on the
     * caller's word and continues the sequence. */
    publisher = OWN(publisher_unchecked)(&ring);
    CHECK(publish_event(&publisher, 99u, 9999u) == 3u, "the unchecked publisher continues at 3");
    expect_event(&ring, &reader, 3u, 99u, 9999u);
    CHECK(OWN(head)(&ring) == 4u, "four entries are published");
    OWN(publisher_release)(&publisher);
}

/* What the log reads back is only what the typed publish wrote; the runtime's own publish
 * writes any words, and a read refuses what no publish of this log wrote. */
static void a_stored_length_above_the_bound_is_refused_and_never_copied(void) {
    static OWN(t) ring;
    OWN(publisher_t) publisher = {0};
    OWN(reader_t) reader;
    OWN(read_result_t) result;
    broadcast_log_conformance_event_t got;
    uint32_t entry[OWN_U(WORDS)];
    CHECK(OWN(init)(&ring), "a log");
    publisher = OWN(publisher_unchecked)(&ring);
    reader = OWN(reader_head)(&ring);

    /* A length of 4 is one past the bound of 3. */
    memset(entry, 0, sizeof entry);
    entry[0] = 4u;
    (void)sce_blogn_publish(&publisher, entry);
    memset(&got, 0xAB, sizeof got);
    result = OWN(read)(&ring, &reader, &got);
    CHECK(result.status == OWN_U(READ_BAD_LENGTH) && result.seq == 0u, "entry 0 is refused for its length");
    CHECK(got.sensor_id == 0xABu && got.value == 0xABABu, "nothing was decoded into the caller's element");

    /* The largest length a word holds is not an index either. */
    entry[0] = UINT32_MAX;
    (void)sce_blogn_publish(&publisher, entry);
    result = OWN(read)(&ring, &reader, &got);
    CHECK(result.status == OWN_U(READ_BAD_LENGTH), "the largest length is refused");

    /* A length of exactly the bound, three bytes 0x0A 0xE8 0x03, is the element sensor 10
     * and value 0x03E8 = 1000, little-endian in the word:
     * 0x0A + 0xE8 * 2^8 + 0x03 * 2^16 = 10 + 59392 + 196608 = 256010. */
    entry[0] = 3u;
    entry[1] = 256010u;
    (void)sce_blogn_publish(&publisher, entry);
    expect_event(&ring, &reader, 2u, 10u, 1000u);

    /* One byte stored is within the bound and is not an element: the codec says so. */
    entry[0] = 1u;
    entry[1] = 0x0Au;
    (void)sce_blogn_publish(&publisher, entry);
    result = OWN(read)(&ring, &reader, &got);
    CHECK(result.status == OWN_U(READ_DECODE_FAILED), "a frame the codec refuses is not an item");
    CHECK(result.codec_status == SCE_FORGE_CODEC_NEED_MORE_BYTES, "the codec wants two more bytes");
    OWN(publisher_release)(&publisher);
}

int main(void) {
    the_narrow_header_states_what_the_document_its_element_and_the_target_give();
    records_come_back_whole_and_are_packed_four_bytes_to_a_word();
    a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest();
    a_position_is_a_residue_and_one_past_the_head_is_the_head();
    the_publishers_place_is_one_whichever_form_takes_it();
    the_ownership_header_states_the_same_row_for_a_target_with_no_swap();
    entries_come_back_through_the_ownership_forms();
    a_stored_length_above_the_bound_is_refused_and_never_copied();
    (void)printf("broadcast log narrow generated (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
