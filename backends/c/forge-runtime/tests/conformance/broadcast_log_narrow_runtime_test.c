/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm of the broadcast-log kind's narrow row (ADR 0062, "Stamp width"): the
 * unit tests of the Rust lane `Narrow<3>`
 * (backends/rust/forge-runtime/src/broadcast_log_core.rs) run against
 * sce/forge/broadcast_log_narrow.h, with the stamp width as the compile-time
 * parameter it is.
 *
 * This translation unit sets SCE_BLOGN_STAMP_BITS to 3 before it includes the
 * header, so the stamp space is M = 8 values, half of it K = 4, and the head, the
 * stamps and the cursors wrap after eight publishes: the cases that a stamp of 32
 * bits could not reach in a run of any length are a few publishes away. The log has
 * 2 slots (a retention of 1) unless a test says otherwise, and an entry is one
 * word holding its true count and not its residue, so a copy that is the entry of
 * the wrong lap shows as a word of another count.
 *
 * The 32-bit row, the ownership forms and real threads are
 * broadcast_log_narrow_runtime_32_test.c's, which keeps the header's own width.
 */

#define SCE_BLOGN_STAMP_BITS 3u

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/forge/broadcast_log_narrow.h>

/* ---- The harness ---- */

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

/* ---- The log of the unit tests ---- */

#define WORDS 1u
#define MOST_SLOTS 4u

/* A log of `slots` slots (2 or 4) of one word, over storage of its own, with its publisher. */
typedef struct fixture {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(MOST_SLOTS, WORDS)];
    sce_blogn_t log;
    sce_blogn_publisher_t publisher;
} fixture_t;

static void open_log(fixture_t *f, uint32_t slots) {
    memset(f, 0, sizeof *f);
    CHECK(sce_blogn_init(&f->log, f->storage, slots, WORDS), "a log of %u slots of one word is built", (unsigned)slots);
    CHECK(sce_blogn_split(&f->log, &f->publisher), "the publisher's place is free in a new log");
}

/* Publishes the entries `from` to `to - 1`: the word of an entry is its true count. */
static void publish_all(fixture_t *f, uint32_t from, uint32_t to) {
    uint32_t count;
    for (count = from; count < to; count++) {
        (void)sce_blogn_publish(&f->publisher, &count);
    }
}

/* One read into a buffer of the sentinel, so a read that wrote nothing is seen. */
static sce_blogn_read_t read_into(fixture_t *f, sce_blogn_reader_t *reader, uint32_t *word) {
    *word = UINT32_MAX;
    return sce_blogn_read(&f->log, reader, word);
}

/* ---- The width gives the stamp space ---- */

static void the_stamp_space_and_its_half_follow_from_the_width(void) {
    fixture_t two;
    fixture_t four;
    CHECK(SCE_BLOGN_MASK == 7u, "2^3 - 1");
    CHECK(SCE_BLOGN_SPACE == 8u, "2^3");
    CHECK(SCE_BLOGN_HALF == 4u, "2^(3 - 1)");
    CHECK(sce_blogn_init(&two.log, two.storage, 2u, WORDS), "two slots");
    CHECK(sce_blogn_init(&four.log, four.storage, 4u, WORDS), "four slots");
    CHECK(sce_blogn_retention(&two.log) == 1u, "2 slots keep 1");
    CHECK(sce_blogn_retention(&four.log) == 3u, "4 slots keep 3");
}

/* done(n) = 2n + 2 mod 8, so n and n + 4 give the same stamp (2n + 10 = 2n + 2 mod 8)
 * and n and n + j for j = 1, 2, 3 differ by 2j = 2, 4, 6. */
static void a_stamp_tells_apart_exactly_half_the_stamp_space(void) {
    uint32_t n;
    uint32_t j;
    for (n = 0; n < 8u; n++) {
        CHECK(sce_blogn_done(n) == sce_blogn_done(n + 4u), "done(%u) one half-space on", (unsigned)n);
        CHECK(sce_blogn_busy(n) == sce_blogn_busy(n + 4u), "busy(%u) one half-space on", (unsigned)n);
        for (j = 1; j < 4u; j++) {
            CHECK(sce_blogn_done(n) != sce_blogn_done(n + j), "done(%u) against +%u", (unsigned)n, (unsigned)j);
            CHECK(sce_blogn_busy(n) != sce_blogn_busy(n + j), "busy(%u) against +%u", (unsigned)n, (unsigned)j);
        }
        CHECK(sce_blogn_busy(n) != sce_blogn_done(n), "busy is odd and done is even");
    }
}

/* The stamps of the last slot, over four laps of the stamp space (the count reduced as
 * a publisher reduces it), are never the value that stands for an unwritten slot. */
static void the_last_slot_is_never_stamped_unwritten(uint32_t slots) {
    uint32_t count;
    for (count = 0; count < 32u; count++) {
        const uint32_t n = count & SCE_BLOGN_MASK;
        if (n % slots == slots - 1u) {
            CHECK(sce_blogn_busy(n) != SCE_BLOGN_UNWRITTEN, "busy(%u), %u slots", (unsigned)n, (unsigned)slots);
            CHECK(sce_blogn_done(n) != SCE_BLOGN_UNWRITTEN, "done(%u), %u slots", (unsigned)n, (unsigned)slots);
        }
    }
}

static void the_unwritten_stamp_is_never_one_the_protocol_writes_into_the_last_slot(void) {
    the_last_slot_is_never_stamped_unwritten(2u);
    the_last_slot_is_never_stamped_unwritten(4u);
}

static void a_stamp_at_the_top_of_the_space_is_what_the_arithmetic_says(void) {
    CHECK(sce_blogn_done(7u) == 0u, "2 * 7 + 2 = 16, 0 modulo 8");
    CHECK(sce_blogn_busy(7u) == 7u, "2 * 7 + 1 = 15, 7 modulo 8");
}

/* ---- The wrap ---- */

/* Twelve entries cross the head's wrap at 8 once. The reader keeps up, so after
 * publish `n` it finds entry `n` and no other: its sequence number is `n mod 8`, and
 * the stored head is `(n + 1) mod 8`. */
static void a_narrow_head_wraps_and_the_reader_still_reads_every_entry_in_order(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    uint32_t word;
    uint32_t n;
    open_log(&f, 2u);
    reader = sce_blogn_reader_head(&f.log);
    CHECK(reader.cursor == 0u, "a reader at the head of a new log holds 0");
    for (n = 0; n < 12u; n++) {
        sce_blogn_read_t result;
        CHECK(sce_blogn_publish(&f.publisher, &n) == n % 8u, "publish %u takes sequence number %u", (unsigned)n,
              (unsigned)(n % 8u));
        CHECK(sce_blogn_head(&f.log) == (n + 1u) % 8u, "the head after %u", (unsigned)n);
        result = read_into(&f, &reader, &word);
        CHECK(result.status == SCE_BLOGN_ITEM && result.seq == n % 8u && word == n, "entry %u", (unsigned)n);
        result = read_into(&f, &reader, &word);
        CHECK(result.status == SCE_BLOGN_EMPTY, "then the head after %u", (unsigned)n);
    }
    CHECK(reader.lost == 0u, "a reader that kept up lost nothing");
}

/* Six entries (0 to 5): the reader opened now is at 6. Three more (6, 7, 8): the head
 * is 9, stored as 9 mod 8 = 1. The reader is (1 - 6) mod 8 = 3 behind, which is the
 * window (1 entry) and 2 more. */
static void a_reader_lapped_across_the_wrap_is_told_the_exact_count(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    publish_all(&f, 0u, 6u);
    reader = sce_blogn_reader_head(&f.log);
    CHECK(reader.cursor == 6u, "the reader holds 6");
    publish_all(&f, 6u, 9u);
    CHECK(sce_blogn_head(&f.log) == 1u, "9 publishes leave the head at 1 modulo 8");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 2u, "the reader is told it missed 2");
    /* 6 + 2 = 8, which is 0 modulo 8: the oldest retained entry is 8. */
    CHECK(reader.cursor == 0u, "the cursor is the residue of the oldest retained entry");
    CHECK(reader.lost == 2u, "and the loss is carried");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 8u,
          "the entry of count 8, whose stamp is the one of count 0 a lap before");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_EMPTY, "and then the head");
}

/* Outside the bound the answer is defined and is what the arithmetic of the cursor
 * says; these four hold each outcome. */

/* Eight entries: the head is 8 mod 8 = 0, where the reader is. It cannot tell that from
 * nothing published, which is outside the bound, and it is told what the arithmetic
 * says. */
static void a_reader_one_stamp_space_behind_is_told_empty_and_keeps_its_place(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    reader = sce_blogn_reader_head(&f.log);
    publish_all(&f, 0u, 8u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_EMPTY, "a reader a stamp space behind is told empty");
    CHECK(reader.cursor == 0u, "its cursor stays");
    CHECK(reader.lost == 0u, "the 8 entries it missed are not counted");
    /* Its place is right: the next entry, count 8, is the one it reads. */
    publish_all(&f, 8u, 9u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 8u, "and it reads entry 8 next");
}

/* Ten entries (0 to 9): the head is 10 mod 8 = 2, so the reader is 2 behind, which is
 * the window (1) and 1 more: skipped is 1, where the truth is 10 - 1 = 9, short by 8. */
static void a_reader_more_than_a_stamp_space_behind_is_lagged_by_the_count_modulo_the_space(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    reader = sce_blogn_reader_head(&f.log);
    publish_all(&f, 0u, 10u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 1u,
          "skipped is the distance modulo 8 less the retention");
    CHECK(reader.lost == 1u, "and that is the loss");
    /* The place is right all the same: 0 + 1 = 1 is the residue of count 9, the oldest
     * retained, and it is read whole. */
    CHECK(reader.cursor == 1u, "the cursor is the oldest retained entry's residue");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 1u && word == 9u, "entry 9 is read whole");
}

/* Nine entries (0 to 8): the head is 9 mod 8 = 1, one ahead of the reader, inside the
 * window; the slot of entry 0 holds entry 8, whose stamp done(8 mod 8) = 2 is done(0):
 * the copy validates and is handed as the entry the reader wanted. This is what the
 * bound exists to keep from happening, and why the deploy states it. */
static void a_reader_a_stamp_space_and_one_behind_is_handed_a_later_lap_as_its_own(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    reader = sce_blogn_reader_head(&f.log);
    publish_all(&f, 0u, 9u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 8u,
          "the entry of a lap on is handed as entry 0");
}

/* A mismatch proves the publisher is `slots` or more ahead. A head that says the
 * reader is only 1 behind is a head one lap round: 1 + 8 = 9 behind, less the
 * retention 1, is 8 skipped, and the cursor (5 + 8) mod 8 = 5 is where it was. */
static void a_head_that_wrapped_during_a_mismatch_still_gives_a_lag_of_at_least_one(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    open_log(&f, 2u);
    reader.cursor = 5u;
    reader.lost = 0u;
    reader.unbound = 0u;
    result = sce_blogn_lagged(&reader, &f.log, 1u);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 8u, "a lag is never reported as skipping nothing");
    CHECK(reader.cursor == 5u, "the cursor is where it was");
    CHECK(reader.lost == 8u, "and the lap is counted");
    /* A head that already says `slots` or more is taken as it is: 3 - 1 = 2. */
    reader.cursor = 5u;
    reader.lost = 0u;
    reader.unbound = 0u;
    result = sce_blogn_lagged(&reader, &f.log, 3u);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 2u, "a distance of 3 less the retention 1");
    CHECK(reader.cursor == 7u, "(5 + 2) mod 8");
}

/* ---- Opening a reader ---- */

static void the_oldest_reader_knows_a_young_log_from_one_that_wrapped(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    /* Every stamp is the unwritten one in a new log, and the last slot's is what the
     * oldest reader asks. */
    CHECK(f.storage[1u * (WORDS + 1u)] == SCE_BLOGN_UNWRITTEN, "the last slot is unwritten");
    /* Nothing published: the oldest retained entry is 0 and the reader is at 0. */
    reader = sce_blogn_reader_oldest(&f.log);
    result = read_into(&f, &reader, &word);
    CHECK(reader.cursor == 0u && result.status == SCE_BLOGN_EMPTY, "an empty log's oldest reader is at the head");
    /* One entry: still `h <= retention = 1`, so the oldest is 0. */
    publish_all(&f, 0u, 1u);
    reader = sce_blogn_reader_oldest(&f.log);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 0u, "after one entry the oldest is entry 0");
    /* Two entries: slot 1 is written, so the oldest retained is 2 - 1 = 1. */
    publish_all(&f, 1u, 2u);
    reader = sce_blogn_reader_oldest(&f.log);
    CHECK(reader.cursor == 1u, "after two entries the oldest is entry 1");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 1u && word == 1u, "and it is read");
    /* After the wrap, with the head at a small residue: 8 entries leave the head at 0,
     * and the oldest retained is count 7, residue (0 - 1) mod 8 = 7. */
    publish_all(&f, 2u, 8u);
    reader = sce_blogn_reader_oldest(&f.log);
    CHECK(reader.cursor == 7u, "with the head at 0 the oldest is residue 7");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 7u && word == 7u, "entry 7");
    /* Nine: the head is 1, the oldest is count 8, residue 0. */
    publish_all(&f, 8u, 9u);
    reader = sce_blogn_reader_oldest(&f.log);
    CHECK(reader.cursor == 0u, "with the head at 1 the oldest is residue 0");
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 8u, "entry 8, by its residue");
}

/* ---- The deferred reader ---- */

/* Both readers are opened on the empty log, one at the oldest now and one deferred; then
 * three entries (0, 1, 2) are published. Retention is 1, so the oldest retained is
 * entry 2. */
static void a_deferred_reader_starts_at_the_oldest_of_its_first_read_and_is_told_of_nothing(void) {
    fixture_t f;
    sce_blogn_reader_t eager;
    sce_blogn_reader_t deferred;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    eager = sce_blogn_reader_oldest(&f.log);
    deferred = sce_blogn_reader_deferred(&f.log);
    CHECK(sce_blogn_reader_is_bound(&eager), "a reader opened at the oldest is bound");
    CHECK(!sce_blogn_reader_is_bound(&deferred), "a deferred reader is not bound before its first read");
    CHECK(deferred.cursor == 0u && deferred.lost == 0u, "no cursor yet reads as 0, with no loss");
    publish_all(&f, 0u, 3u);
    /* The eager reader holds 0, which is 3 behind: it is told it missed 2. */
    result = read_into(&f, &eager, &word);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 2u, "the eager reader is told of the lag");
    /* The deferred reader binds at this read, at the oldest retained entry. */
    result = read_into(&f, &deferred, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 2u && word == 2u,
          "the first read of a deferred reader is the oldest retained entry");
    CHECK(sce_blogn_reader_is_bound(&deferred), "the first read binds it");
    CHECK(deferred.lost == 0u, "a reader that waited is told of nothing");
    CHECK(deferred.cursor == 3u, "and holds the entry after the one it read");
    result = read_into(&f, &deferred, &word);
    CHECK(result.status == SCE_BLOGN_EMPTY, "then the head");
}

static void the_other_constructors_give_bound_readers(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    open_log(&f, 2u);
    reader = sce_blogn_reader_oldest(&f.log);
    CHECK(sce_blogn_reader_is_bound(&reader), "oldest is bound");
    reader = sce_blogn_reader_head(&f.log);
    CHECK(sce_blogn_reader_is_bound(&reader), "head is bound");
    reader = sce_blogn_reader_at(&f.log, 0u);
    CHECK(sce_blogn_reader_is_bound(&reader), "at is bound");
}

static void a_deferred_reader_on_a_log_with_nothing_published_is_empty_and_then_reads_from_the_start(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    reader = sce_blogn_reader_deferred(&f.log);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_EMPTY, "nothing published is empty");
    CHECK(sce_blogn_reader_is_bound(&reader), "the empty read bound it");
    CHECK(reader.cursor == 0u, "at the start of the log");
    publish_all(&f, 0u, 1u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 0u, "the first entry is read from the start");
    CHECK(reader.lost == 0u, "with nothing lost");
}

/* Ten entries cross the head's wrap at 8, so the head is 2 and the last slot is written.
 * The oldest retained is entry 9, cursor 9 mod 8 = 1. */
static void a_deferred_reader_across_the_wrap_binds_to_the_oldest_by_the_last_slot(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 2u);
    publish_all(&f, 0u, 10u);
    CHECK(sce_blogn_head(&f.log) == 2u, "ten publishes leave the head at 2 modulo 8");
    reader = sce_blogn_reader_deferred(&f.log);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 1u && word == 9u, "the oldest retained is entry 9");
    CHECK(reader.lost == 0u, "and nothing is lost");
    /* A reader that falls behind later is told as any reader is: three more entries leave
     * the head at 5, the reader holding 2 is 3 behind, and misses 3 - 1 = 2. */
    publish_all(&f, 10u, 13u);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == 2u, "a later lag is told");
    CHECK(reader.lost == 2u && reader.cursor == 4u, "and counted, at the residue of entry 12");
}

/* Three entries of a log that retains three: its last slot was never written, so the
 * oldest is entry 0 though the head is 3. */
static void a_deferred_reader_on_a_young_log_binds_to_entry_zero_by_the_last_slot(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    uint32_t word;
    open_log(&f, 4u);
    publish_all(&f, 0u, 3u);
    CHECK(f.storage[3u * (WORDS + 1u)] == SCE_BLOGN_UNWRITTEN, "the last slot is still unwritten");
    reader = sce_blogn_reader_deferred(&f.log);
    result = read_into(&f, &reader, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u && word == 0u, "the oldest is entry 0");
    CHECK(reader.lost == 0u, "and nothing is lost");
}

/* The bind and sce_blogn_reader_oldest give one cursor, whatever the log has published
 * and whichever side of the wrap the head is. */
static void a_deferred_reader_binds_where_the_oldest_reader_opens(void) {
    uint32_t slots;
    for (slots = 2u; slots <= 4u; slots += 2u) {
        fixture_t f;
        uint32_t count;
        uint32_t word;
        open_log(&f, slots);
        for (count = 0; count < 20u; count++) {
            sce_blogn_reader_t deferred = sce_blogn_reader_deferred(&f.log);
            const sce_blogn_reader_t oldest = sce_blogn_reader_oldest(&f.log);
            const sce_blogn_read_t result = read_into(&f, &deferred, &word);
            if (count == 0u) {
                CHECK(result.status == SCE_BLOGN_EMPTY, "%u slots: an empty log is empty to a deferred reader",
                      (unsigned)slots);
            } else {
                CHECK(result.status == SCE_BLOGN_ITEM && result.seq == oldest.cursor, "%u slots, %u publishes",
                      (unsigned)slots, (unsigned)count);
            }
            CHECK(deferred.lost == 0u && sce_blogn_reader_is_bound(&deferred),
                  "%u slots, %u publishes: bound, none lost", (unsigned)slots, (unsigned)count);
            publish_all(&f, count, count + 1u);
        }
    }
}

/* The head is 6; K = 4. A position fewer than 4 behind is taken as it is. */
static void a_position_is_read_modulo_the_stamp_space_and_one_ahead_is_the_head(void) {
    fixture_t f;
    open_log(&f, 2u);
    publish_all(&f, 0u, 6u);
    CHECK(sce_blogn_reader_at(&f.log, 3u).cursor == 3u, "(6 - 3) mod 8 = 3 < 4");
    CHECK(sce_blogn_reader_at(&f.log, 6u).cursor == 6u, "at the head");
    /* 4 or more behind is what one ahead of the head looks like, and is the head:
     * (6 - 2) mod 8 = 4, and (6 - 7) mod 8 = 7. */
    CHECK(sce_blogn_reader_at(&f.log, 2u).cursor == 6u, "4 behind is the head");
    CHECK(sce_blogn_reader_at(&f.log, 7u).cursor == 6u, "one ahead is the head");
    /* A count past the stamp space is reduced: 14 mod 8 = 6. */
    CHECK(sce_blogn_reader_at(&f.log, 14u).cursor == 6u, "a count past the stamp space is reduced");
    CHECK(sce_blogn_reader_at(&f.log, UINT64_C(0x100000006)).cursor == 6u, "a wide count is reduced as well");
}

/* ---- The slot count ---- */

/* 3 retained of 4 slots over 8 stamp values: ten entries, read one at a time, then a
 * reader that was held back for 3. */
static void a_log_with_as_many_slots_as_half_the_stamp_space_keeps_its_window_across_the_wrap(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    sce_blogn_reader_t held;
    sce_blogn_reader_t behind;
    sce_blogn_read_t result;
    uint32_t word;
    uint32_t n;
    open_log(&f, 4u);
    reader = sce_blogn_reader_head(&f.log);
    for (n = 0; n < 10u; n++) {
        publish_all(&f, n, n + 1u);
        result = read_into(&f, &reader, &word);
        CHECK(result.status == SCE_BLOGN_ITEM && result.seq == n % 8u && word == n, "entry %u", (unsigned)n);
    }
    publish_all(&f, 10u, 13u);
    /* A reader that waited through 3 publishes is 3 behind the head, inside the window
     * of 3, and reads the oldest retained entry, count 10, first. */
    held = sce_blogn_reader_head(&f.log);
    CHECK(held.cursor == 13u % 8u, "the head is 13 modulo 8");
    behind.cursor = 10u % 8u;
    behind.lost = 0u;
    behind.unbound = 0u;
    result = read_into(&f, &behind, &word);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 2u && word == 10u,
          "(5 - 2) mod 8 = 3 behind: in the window");
    result = read_into(&f, &held, &word);
    CHECK(result.status == SCE_BLOGN_EMPTY, "the reader at the head sees nothing");
}

/* The slot count must divide the stamp space: a power of two, at most K. */
static void the_constructor_refuses_a_slot_count_the_stamp_cannot_number(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(8u, WORDS)];
    sce_blogn_t log;
    CHECK(!sce_blogn_init(&log, storage, 3u, WORDS), "3 slots do not divide 8");
    CHECK(!sce_blogn_init(&log, storage, 6u, WORDS), "6 slots do not divide 8");
    CHECK(!sce_blogn_init(&log, storage, 8u, WORDS), "8 slots are more than half the stamp space");
    CHECK(!sce_blogn_init(&log, storage, 1u, WORDS), "one slot would retain nothing");
    CHECK(!sce_blogn_init(&log, storage, 0u, WORDS), "no slot would retain nothing");
    CHECK(!sce_blogn_init(&log, storage, 2u, 0u), "an entry of no word carries nothing");
    CHECK(!sce_blogn_init(&log, NULL, 2u, WORDS), "a log needs storage");
    CHECK(sce_blogn_init(&log, storage, 2u, WORDS), "two slots retain one entry");
    CHECK(sce_blogn_init(&log, storage, 4u, WORDS), "four slots are half the stamp space and are kept");
    CHECK(sce_blogn_retention(&log) == 3u, "and the retention is three");
}

/* An entry of three words comes back whole across the wrap, each word of its own. */
static void an_entry_of_several_words_comes_back_whole(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(2u, 3u)];
    sce_blogn_t log;
    sce_blogn_publisher_t publisher = {0};
    sce_blogn_reader_t reader;
    uint32_t n;
    CHECK(sce_blogn_init(&log, storage, 2u, 3u), "two slots of three words");
    CHECK(sce_blogn_split(&log, &publisher), "the publisher");
    reader = sce_blogn_reader_head(&log);
    for (n = 0; n < 10u; n++) {
        uint32_t entry[3];
        uint32_t out[3] = {0u, 0u, 0u};
        sce_blogn_read_t result;
        entry[0] = n + 1u;
        entry[1] = n + 100u;
        entry[2] = UINT32_C(0xDEADBEEF) - n;
        (void)sce_blogn_publish(&publisher, entry);
        result = sce_blogn_read(&log, &reader, out);
        CHECK(result.status == SCE_BLOGN_ITEM && result.seq == n % 8u, "entry %u is an item", (unsigned)n);
        CHECK(out[0] == n + 1u && out[1] == n + 100u && out[2] == UINT32_C(0xDEADBEEF) - n, "entry %u is whole",
              (unsigned)n);
    }
}

/* ---- The publisher ---- */

static void a_split_publisher_holds_the_place_and_its_successor_continues_the_sequence(void) {
    fixture_t f;
    sce_blogn_publisher_t other = {0};
    uint32_t entry = 0u;
    memset(&f, 0, sizeof f);
    CHECK(sce_blogn_init(&f.log, f.storage, 2u, WORDS), "a log");
    CHECK(sce_blogn_split(&f.log, &f.publisher), "the place is free");
    CHECK(!sce_blogn_split(&f.log, &other), "a second place is refused by the ownership form");
    CHECK(!sce_blogn_publisher_acquire(&f.log, &other), "and by the swap");
    CHECK(sce_blogn_publish(&f.publisher, &entry) == 0u, "the first publisher publishes entry 0");
    entry = 1u;
    CHECK(sce_blogn_publish(&f.publisher, &entry) == 1u, "and entry 1");
    sce_blogn_publisher_release(&f.publisher);
    /* The place is free again, and the successor continues where the head says. */
    CHECK(sce_blogn_publisher_acquire(&f.log, &other), "the swap takes the freed place");
    entry = 2u;
    CHECK(sce_blogn_publish(&other, &entry) == 2u, "the sequence continues");
    CHECK(!sce_blogn_split(&f.log, &f.publisher), "a place taken by the swap refuses the ownership form");
    sce_blogn_publisher_release(&other);
    /* A log in static storage takes it on the caller's word, and the flag is raised. */
    other = sce_blogn_publisher_unchecked(&f.log);
    entry = 3u;
    CHECK(sce_blogn_publish(&other, &entry) == 3u, "the unchecked publisher continues the sequence");
    CHECK(!sce_blogn_publisher_acquire(&f.log, &f.publisher), "and holds the place against the swap");
    sce_blogn_publisher_release(&other);
    CHECK(sce_blogn_publisher_acquire(&f.log, &f.publisher), "released, the place is free");
}

/* A narrow count has no horizon: 100 entries are 12 laps of the stamp space and 4
 * entries, and the reader that keeps up reads each by its residue. */
static void publishing_forever_does_not_reach_a_horizon_in_a_narrow_log(void) {
    fixture_t f;
    sce_blogn_reader_t reader;
    uint32_t word;
    uint32_t n;
    open_log(&f, 2u);
    reader = sce_blogn_reader_head(&f.log);
    for (n = 0; n < 100u; n++) {
        sce_blogn_read_t result;
        publish_all(&f, n, n + 1u);
        result = read_into(&f, &reader, &word);
        CHECK(result.status == SCE_BLOGN_ITEM && result.seq == n % 8u && word == n, "entry %u", (unsigned)n);
    }
    CHECK(reader.lost == 0u, "nothing was lost");
}

int main(void) {
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
    the_constructor_refuses_a_slot_count_the_stamp_cannot_number();
    an_entry_of_several_words_comes_back_whole();
    a_split_publisher_holds_the_place_and_its_successor_continues_the_sequence();
    publishing_forever_does_not_reach_a_horizon_in_a_narrow_log();
    (void)printf("broadcast log narrow runtime (C11, 3-bit stamp): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
