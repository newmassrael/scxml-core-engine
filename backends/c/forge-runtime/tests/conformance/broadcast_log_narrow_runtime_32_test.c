/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm of the broadcast-log kind's narrow row (ADR 0062, "Stamp width"), at
 * the width a row keeps: sce/forge/broadcast_log_narrow.h with its own
 * SCE_BLOGN_STAMP_BITS, 32. The wrap cases are broadcast_log_narrow_runtime_test.c's,
 * which sets a stamp of 3 bits because a stamp of 32 cannot reach them; this file
 * holds what only the full width shows (the arithmetic at the top of a 32-bit space,
 * a word that keeps all its bits, a lag counted one lap on in 64 bits), the
 * constructor's refusals, and real threads: one publisher and several readers, where
 * every entry a reader is handed must be whole and the entries it is handed and the
 * entries it is told it missed must add up to every entry published.
 *
 * An entry is three words, all of them the sequence number plus one: a copy that
 * mixes two entries shows as words that differ, and a copy of a slot that nothing
 * was written to shows as a word of zero, which no entry has.
 */

#include <pthread.h>
#include <sched.h>
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

#define WORDS 3u

typedef struct entry {
    uint32_t words[WORDS];
} entry_t;

static entry_t entry_of(uint32_t seq) {
    entry_t e;
    unsigned k;
    for (k = 0; k < WORDS; k++) {
        e.words[k] = seq + 1u;
    }
    return e;
}

static int is_whole_entry(const uint32_t *words, uint32_t seq) {
    unsigned k;
    for (k = 0; k < WORDS; k++) {
        if (words[k] != seq + 1u) {
            return 0;
        }
    }
    return 1;
}

/* ---- The row's width ---- */

static void the_row_keeps_a_stamp_of_32_bits(void) {
    CHECK(SCE_BLOGN_STAMP_BITS == 32u, "a row keeps a stamp of 32 bits");
    CHECK(SCE_BLOGN_MASK == UINT32_MAX, "the stamp is the whole word");
    CHECK(SCE_BLOGN_SPACE == UINT64_C(0x100000000), "2^32 values");
    CHECK(SCE_BLOGN_HALF == UINT32_C(0x80000000), "a stamp tells 2^31 entries of one slot apart");
    CHECK(sizeof(((sce_blogn_t *)0)->head) == 4u, "the head is a 32-bit word");
    CHECK(sizeof(((sce_blogn_reader_t *)0)->cursor) == 4u, "so is a cursor");
}

/* done: entries 2^31 - 1 and 2^32 - 1 (half a stamp space apart) have the same stamps.
 * 2 * (2^31 - 1) + 2 = 2^32 and 2 * (2^32 - 1) + 2 = 2^33, both 0 modulo 2^32. busy:
 * 2 * (2^31 - 1) + 1 = 2^32 - 1 and 2 * (2^32 - 1) + 1 = 2^33 - 1, both 2^32 - 1. */
static void a_stamp_tells_apart_half_of_a_32_bit_space(void) {
    CHECK(sce_blogn_done(0x7FFFFFFFu) == 0u, "done(2^31 - 1)");
    CHECK(sce_blogn_done(0xFFFFFFFFu) == 0u, "done(2^32 - 1)");
    CHECK(sce_blogn_done(0u) == 2u, "done(0)");
    CHECK(sce_blogn_busy(0x7FFFFFFFu) == 0xFFFFFFFFu, "busy(2^31 - 1)");
    CHECK(sce_blogn_busy(0xFFFFFFFFu) == 0xFFFFFFFFu, "busy(2^32 - 1)");
    CHECK(sce_blogn_busy(0u) == 1u, "busy(0)");
}

/* A lag counted from a head that wrapped is one lap on, and a lap is 2^32 entries:
 * the count does not fit 32 bits, which is why `skipped` is 64. */
static void a_lag_from_a_wrapped_head_is_counted_a_lap_on_in_64_bits(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(4u, 1u)];
    sce_blogn_t log;
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    CHECK(sce_blogn_init(&log, storage, 4u, 1u), "four slots");
    reader.cursor = 10u;
    reader.lost = 0u;
    /* 1 + 2^32 behind, less the retention 3: the head says the reader is 1 behind, which
     * is inside the window of 4 slots and so cannot be what a lag is answered from. */
    result = sce_blogn_lagged(&reader, &log, 1u);
    CHECK(result.status == SCE_BLOGN_LAGGED && result.skipped == UINT64_C(4294967294), "2^32 + 1 - 3");
    CHECK(reader.cursor == 8u, "(10 + 2^32 - 2) mod 2^32 is the residue of the oldest retained entry, 11 - 3");
    CHECK(reader.lost == UINT64_C(4294967294), "and the loss is the count");
    /* A distance of 3 is inside the window of 4 slots as well; the lap added to it is the
     * fewest it can be, 2^32 + 3 - 3. */
    reader.cursor = 10u;
    reader.lost = 0u;
    result = sce_blogn_lagged(&reader, &log, 3u);
    CHECK(result.skipped == UINT64_C(4294967296), "2^32 itself, which 32 bits cannot hold");
    CHECK(reader.cursor == 10u, "and the cursor is where it was");
    /* 4 behind is outside the window of 3 and is taken as it is: 4 - 3. */
    reader.cursor = 10u;
    reader.lost = 0u;
    result = sce_blogn_lagged(&reader, &log, 4u);
    CHECK(result.skipped == 1u && reader.cursor == 11u, "a distance of the slot count is a lag of 1");
}

static void a_word_keeps_all_of_its_32_bits(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(4u, 2u)];
    sce_blogn_t log;
    sce_blogn_publisher_t publisher = {0};
    sce_blogn_reader_t reader;
    sce_blogn_read_t result;
    const uint32_t entry[2] = {UINT32_C(0xDEADBEEF), UINT32_C(0x01020304)};
    uint32_t out[2] = {0u, 0u};
    CHECK(sce_blogn_init(&log, storage, 4u, 2u), "four slots of two words");
    CHECK(sce_blogn_split(&log, &publisher), "the publisher");
    CHECK(sce_blogn_publish(&publisher, entry) == 0u, "entry 0");
    reader = sce_blogn_reader_oldest(&log);
    result = sce_blogn_read(&log, &reader, out);
    CHECK(result.status == SCE_BLOGN_ITEM && result.seq == 0u, "it is read back");
    CHECK(out[0] == UINT32_C(0xDEADBEEF) && out[1] == UINT32_C(0x01020304), "each word keeps all of its 32 bits");
}

/* ---- The constructor ---- */

static void the_constructor_refuses_a_shape_it_cannot_run(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(8u, 2u)];
    sce_blogn_t log;
    CHECK(!sce_blogn_init(&log, storage, 1u, 2u), "one slot would retain nothing");
    CHECK(!sce_blogn_init(&log, storage, 0u, 2u), "no slot would retain nothing");
    CHECK(!sce_blogn_init(&log, storage, 2u, 0u), "an entry of no word carries nothing");
    CHECK(!sce_blogn_init(&log, NULL, 2u, 2u), "a log needs storage");
    CHECK(!sce_blogn_init(&log, storage, 3u, 2u), "3 slots do not divide 2^32");
    CHECK(!sce_blogn_init(&log, storage, 7u, 2u), "7 slots do not divide 2^32");
    CHECK(!sce_blogn_init(&log, storage, UINT32_C(0xC0000000), 2u), "a slot count that is not a power of two");
    CHECK(sce_blogn_init(&log, storage, 2u, 2u), "two slots retain one entry");
    CHECK(sce_blogn_retention(&log) == 1u, "and the retention is one");
    CHECK(sce_blogn_init(&log, storage, 8u, 2u), "eight slots retain seven");
    CHECK(sce_blogn_retention(&log) == 7u, "and the retention is seven");
}

/* Every slot is stamped unwritten, and the words are zero. */
static void a_new_log_is_stamped_unwritten_and_empty(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(4u, 2u)];
    sce_blogn_t log;
    sce_blogn_reader_t oldest;
    uint32_t out[2];
    uint32_t slot;
    unsigned k;
    memset(storage, 0xFF, sizeof storage);
    CHECK(sce_blogn_init(&log, storage, 4u, 2u), "four slots of two words");
    for (slot = 0; slot < 4u; slot++) {
        CHECK(storage[slot * 3u] == SCE_BLOGN_UNWRITTEN, "slot %u is stamped unwritten", (unsigned)slot);
        for (k = 1u; k < 3u; k++) {
            CHECK(storage[slot * 3u + k] == 0u, "slot %u word %u is zero", (unsigned)slot, k);
        }
    }
    CHECK(sce_blogn_head(&log) == 0u, "nothing is published");
    oldest = sce_blogn_reader_oldest(&log);
    CHECK(oldest.cursor == 0u && sce_blogn_read(&log, &oldest, out).status == SCE_BLOGN_EMPTY,
          "the oldest reader of a new log is at its head");
}

/* ---- The publisher ---- */

/* The three ways to take the place are one place: each refuses the others while it is
 * held, a swap-less target uses the ownership forms, and the successor continues. */
static void the_publishers_forms_are_one_place(void) {
    uint32_t storage[SCE_BLOGN_STORAGE_WORDS(2u, WORDS)];
    sce_blogn_t log;
    sce_blogn_publisher_t first = {0};
    sce_blogn_publisher_t second = {0};
    entry_t e;
    CHECK(sce_blogn_init(&log, storage, 2u, WORDS), "a log");
    CHECK(sce_blogn_publisher_acquire(&log, &first), "the swap takes the place");
    CHECK(!sce_blogn_publisher_acquire(&log, &second), "a second swap is refused");
    CHECK(!sce_blogn_split(&log, &second), "and so is the ownership form");
    e = entry_of(0u);
    CHECK(sce_blogn_publish(&first, e.words) == 0u, "entry 0");
    sce_blogn_publisher_release(&first);
    CHECK(sce_blogn_split(&log, &second), "the ownership form takes the freed place");
    CHECK(!sce_blogn_publisher_acquire(&log, &first), "which the swap then finds taken");
    e = entry_of(1u);
    CHECK(sce_blogn_publish(&second, e.words) == 1u, "the sequence continues");
    sce_blogn_publisher_release(&second);
    first = sce_blogn_publisher_unchecked(&log);
    e = entry_of(2u);
    CHECK(sce_blogn_publish(&first, e.words) == 2u, "the unchecked publisher continues it too");
    CHECK(!sce_blogn_split(&log, &second), "and holds the place against the ownership form");
    sce_blogn_publisher_release(&first);
}

/* ---- Real threads ---- */

#define STRESS_ENTRIES 200000u
#define STRESS_READERS 3u
#define STRESS_MOST_SLOTS 8u
#define DEADLINE_SPINS 400000000ull

typedef struct stress {
    sce_blogn_t log;
    uint32_t total;
    uint32_t failed;
    pthread_t publisher_thread;
} stress_t;

typedef struct stress_reader {
    stress_t *stress;
    pthread_t thread;
    uint64_t items;
    uint64_t lost;
    uint64_t lags;
    uint64_t torn;
    uint64_t out_of_order;
    uint64_t bad_lag;
    int timed_out;
} stress_reader_t;

static void *stress_publish(void *arg) {
    stress_t *s = (stress_t *)arg;
    sce_blogn_publisher_t publisher = {0};
    uint32_t n;
    if (!sce_blogn_publisher_acquire(&s->log, &publisher)) {
        s->failed = 1u;
        return NULL;
    }
    for (n = 0; n < s->total; n++) {
        const entry_t e = entry_of(n);
        if (sce_blogn_publish(&publisher, e.words) != n) {
            s->failed = 1u;
            break;
        }
        /* Now and then give the readers the processor, so that they are sometimes
         * ahead of the publisher and sometimes far behind it. */
        if (n % 1024u == 1023u) {
            (void)sched_yield();
        }
    }
    sce_blogn_publisher_release(&publisher);
    return NULL;
}

static void *stress_read(void *arg) {
    stress_reader_t *r = (stress_reader_t *)arg;
    stress_t *s = r->stress;
    sce_blogn_reader_t reader = sce_blogn_reader_at(&s->log, 0u);
    uint32_t last_item_plus_one = 0;
    uint64_t spins = 0;
    /* The count does not wrap in a run of this length, so a cursor is the count. */
    while (reader.cursor < s->total) {
        uint32_t out[WORDS];
        const uint32_t before = reader.cursor;
        const uint64_t lost_before = reader.lost;
        const sce_blogn_read_t result = sce_blogn_read(&s->log, &reader, out);
        if (result.status == SCE_BLOGN_ITEM) {
            spins = 0;
            r->items++;
            if (result.seq != before || reader.cursor != before + 1u || reader.lost != lost_before ||
                !is_whole_entry(out, result.seq)) {
                r->torn++;
            }
            if (result.seq < last_item_plus_one) {
                r->out_of_order++;
            }
            last_item_plus_one = result.seq + 1u;
        } else if (result.status == SCE_BLOGN_LAGGED) {
            spins = 0;
            r->lags++;
            if (result.skipped < 1u || reader.cursor != before + result.skipped ||
                reader.lost != lost_before + result.skipped) {
                r->bad_lag++;
            }
        } else {
            if (reader.cursor != before) {
                r->bad_lag++;
            }
            if (++spins > DEADLINE_SPINS) {
                r->timed_out = 1;
                return NULL;
            }
            (void)sched_yield();
        }
    }
    r->lost = reader.lost;
    return NULL;
}

static void one_publisher_and_several_readers_see_whole_entries(uint32_t slots) {
    /* Static storage for the largest log below, which is rebuilt for each. */
    static uint32_t pool[SCE_BLOGN_STORAGE_WORDS(STRESS_MOST_SLOTS, WORDS)];
    stress_t s;
    stress_reader_t readers[STRESS_READERS];
    unsigned i;
    memset(&s, 0, sizeof s);
    memset(readers, 0, sizeof readers);
    CHECK(slots <= STRESS_MOST_SLOTS, "the pool holds %u slots", (unsigned)STRESS_MOST_SLOTS);
    CHECK(sce_blogn_init(&s.log, pool, slots, WORDS), "a log of %u slots", (unsigned)slots);
    s.total = STRESS_ENTRIES;
    for (i = 0; i < STRESS_READERS; i++) {
        readers[i].stress = &s;
        (void)pthread_create(&readers[i].thread, NULL, stress_read, &readers[i]);
    }
    (void)pthread_create(&s.publisher_thread, NULL, stress_publish, &s);
    (void)pthread_join(s.publisher_thread, NULL);
    for (i = 0; i < STRESS_READERS; i++) {
        (void)pthread_join(readers[i].thread, NULL);
    }
    CHECK(s.failed == 0u, "the publisher took its place and every sequence number in order");
    CHECK(sce_blogn_head(&s.log) == STRESS_ENTRIES, "every entry was published");
    for (i = 0; i < STRESS_READERS; i++) {
        CHECK(!readers[i].timed_out, "slots %u reader %u finished before the deadline", (unsigned)slots, i);
        CHECK(readers[i].torn == 0u, "slots %u reader %u: %llu entries not whole or not at the cursor", (unsigned)slots,
              i, (unsigned long long)readers[i].torn);
        CHECK(readers[i].out_of_order == 0u, "slots %u reader %u: %llu entries out of order", (unsigned)slots, i,
              (unsigned long long)readers[i].out_of_order);
        CHECK(readers[i].bad_lag == 0u,
              "slots %u reader %u: %llu lags that skipped nothing or moved the cursor wrongly", (unsigned)slots, i,
              (unsigned long long)readers[i].bad_lag);
        /* Every sequence number below the total was handed over or counted lost. */
        CHECK(readers[i].items + readers[i].lost == STRESS_ENTRIES,
              "slots %u reader %u: %llu items and %llu lost make %llu entries", (unsigned)slots, i,
              (unsigned long long)readers[i].items, (unsigned long long)readers[i].lost,
              (unsigned long long)(readers[i].items + readers[i].lost));
    }
}

static void real_threads_see_whole_entries_and_lose_only_what_they_are_told(void) {
    one_publisher_and_several_readers_see_whole_entries(2u);
    one_publisher_and_several_readers_see_whole_entries(4u);
    one_publisher_and_several_readers_see_whole_entries(8u);
}

int main(void) {
    the_row_keeps_a_stamp_of_32_bits();
    a_stamp_tells_apart_half_of_a_32_bit_space();
    a_lag_from_a_wrapped_head_is_counted_a_lap_on_in_64_bits();
    a_word_keeps_all_of_its_32_bits();
    the_constructor_refuses_a_shape_it_cannot_run();
    a_new_log_is_stamped_unwritten_and_empty();
    the_publishers_forms_are_one_place();
    real_threads_see_whole_entries_and_lose_only_what_they_are_told();
    (void)printf("broadcast log narrow runtime (C11, 32-bit stamp): %u check(s), %u failure(s)\n", g_checks,
                 g_failures);
    return g_failures == 0u ? 0 : 1;
}
