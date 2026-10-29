/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm of the `broadcast-log` kind's layer 1 (RFC §synth-5-S, ADR 0062): the unit tests of
 * the Rust row (backends/rust/forge-runtime/src/broadcast_log.rs) run against
 * sce/forge/broadcast_log.h, the refusals of the constructor, and real threads:
 * one publisher and several readers, where every entry a reader is handed must be
 * whole and the entries it is handed and the entries it is told it missed must
 * add up to every entry published.
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

#include <sce/forge/broadcast_log.h>

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

/* ---- The entry ---- */

#define WORDS 3u
/* Three slots retain two entries, as the Rust row's unit tests do. */
#define SLOTS 3u

typedef struct entry {
    uint64_t words[WORDS];
} entry_t;

static entry_t entry_of(uint64_t seq) {
    entry_t e;
    unsigned k;
    for (k = 0; k < WORDS; k++) {
        e.words[k] = seq + 1u;
    }
    return e;
}

static int is_whole_entry(const uint64_t *words, uint64_t seq) {
    unsigned k;
    for (k = 0; k < WORDS; k++) {
        if (words[k] != seq + 1u) {
            return 0;
        }
    }
    return 1;
}

/* A log of the unit tests' shape, over storage of its own. */
typedef struct fixture {
    uint64_t storage[SCE_BLOG_STORAGE_WORDS(SLOTS, WORDS)];
    sce_blog_t log;
    sce_blog_publisher_t publisher;
} fixture_t;

static void fixture_open(fixture_t *f) {
    CHECK(sce_blog_init(&f->log, f->storage, SLOTS, WORDS), "a log of %u slots of %u words is built", SLOTS, WORDS);
}

static void publish_through(fixture_t *f, uint64_t count) {
    uint64_t n;
    for (n = 0; n < count; n++) {
        const entry_t e = entry_of(n);
        CHECK(sce_blog_publish(&f->publisher, e.words) == n, "publish %llu takes sequence number %llu",
              (unsigned long long)n, (unsigned long long)n);
    }
}

/* One read into a buffer of the sentinel, so a read that wrote nothing is seen. */
static sce_blog_read_t read_into(fixture_t *f, sce_blog_reader_t *reader, entry_t *out) {
    unsigned k;
    for (k = 0; k < WORDS; k++) {
        out->words[k] = UINT64_MAX;
    }
    return sce_blog_read(&f->log, reader, out->words);
}

/* ---- The unit tests of the Rust row ---- */

static void an_empty_log_is_empty_to_every_reader(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t reader;
    sce_blog_read_t result;
    fixture_open(&f);
    reader = sce_blog_reader_oldest(&f.log);
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_EMPTY, "an empty log reads as empty");
    CHECK(reader.cursor == 0u, "an empty read moves nothing, the cursor is %llu", (unsigned long long)reader.cursor);
    CHECK(sce_blog_head(&f.log) == 0u, "an empty log has published nothing");
    CHECK(sce_blog_retention(&f.log) == SLOTS - 1u, "the retention is one less than the slots");
}

static void entries_come_back_in_order_with_their_words(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t reader;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the first publisher");
    reader = sce_blog_reader_oldest(&f.log);
    publish_through(&f, 2u);
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 0u, "the first read is entry 0");
    CHECK(is_whole_entry(out.words, 0u), "entry 0 comes back with its words");
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 1u, "the second read is entry 1");
    CHECK(is_whole_entry(out.words, 1u), "entry 1 comes back with its words");
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_EMPTY, "then the head");
    CHECK(reader.lost == 0u, "a reader that kept up lost nothing");
    CHECK(reader.cursor == 2u, "the cursor moved past both entries");
    sce_blog_publisher_release(&f.publisher);
}

static void the_window_is_exactly_the_retention(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t behind;
    sce_blog_reader_t oldest;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 3u);
    /* h = 3: the window is [1, 3), so a reader at 0 is one behind it. */
    behind = sce_blog_reader_at(&f.log, 0u);
    result = read_into(&f, &behind, &out);
    CHECK(result.status == SCE_BLOG_LAGGED && result.skipped == 1u, "a reader at 0 is one behind the window");
    CHECK(behind.cursor == 1u, "a lag moves the cursor to the oldest retained entry");
    CHECK(behind.lost == 1u, "a lag adds what it skipped to the reader's total");
    result = read_into(&f, &behind, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 1u && is_whole_entry(out.words, 1u),
          "after a lag the reader reads the oldest retained entry");
    /* A reader at the oldest retained entry lost nothing. */
    oldest = sce_blog_reader_oldest(&f.log);
    CHECK(oldest.cursor == 1u, "the oldest reader starts at the start of the window");
    result = read_into(&f, &oldest, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 1u && is_whole_entry(out.words, 1u),
          "a reader at the oldest reads it");
    sce_blog_publisher_release(&f.publisher);
}

static void readers_do_not_share_a_cursor_or_a_loss(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t a;
    sce_blog_reader_t b;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 5u);
    a = sce_blog_reader_at(&f.log, 0u);
    b = sce_blog_reader_at(&f.log, 3u);
    result = read_into(&f, &a, &out);
    CHECK(result.status == SCE_BLOG_LAGGED && result.skipped == 3u, "a reader at 0 of 5 is 3 behind a window of 2");
    result = read_into(&f, &b, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 3u && is_whole_entry(out.words, 3u),
          "the other reader reads entry 3");
    CHECK(a.lost == 3u && b.lost == 0u, "each reader has its own loss");
    CHECK(b.cursor == 4u, "and its own cursor");
    sce_blog_publisher_release(&f.publisher);
}

static void a_reader_at_the_head_sees_only_what_comes_after(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t reader;
    sce_blog_reader_t past;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 1u);
    reader = sce_blog_reader_head(&f.log);
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_EMPTY, "a reader at the head sees nothing yet");
    {
        const entry_t e = entry_of(1u);
        CHECK(sce_blog_publish(&f.publisher, e.words) == 1u, "the second entry");
    }
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 1u && is_whole_entry(out.words, 1u),
          "then it sees the entry published after it opened");
    /* A position past the head is clamped to it. */
    past = sce_blog_reader_at(&f.log, 99u);
    CHECK(past.cursor == 2u, "a position past the head is the head");
    sce_blog_publisher_release(&f.publisher);
}

static void there_is_one_publisher_and_its_successor_continues_the_sequence(void) {
    fixture_t f;
    sce_blog_publisher_t second;
    entry_t e;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the first publisher");
    CHECK(!sce_blog_publisher_acquire(&f.log, &second), "a second publisher is refused");
    e = entry_of(0u);
    CHECK(sce_blog_publish(&f.publisher, e.words) == 0u, "the first publisher publishes entry 0");
    sce_blog_publisher_release(&f.publisher);
    CHECK(sce_blog_publisher_acquire(&f.log, &second), "the place was freed");
    e = entry_of(1u);
    CHECK(sce_blog_publish(&second, e.words) == 1u, "its successor continues the sequence");
    sce_blog_publisher_release(&second);
}

static void publishing_past_the_window_keeps_the_newest_entries(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t reader;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 100u);
    reader = sce_blog_reader_oldest(&f.log);
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 98u && is_whole_entry(out.words, 98u),
          "entry 98 is retained");
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 99u && is_whole_entry(out.words, 99u),
          "entry 99 is retained");
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_EMPTY, "and then the head");
    sce_blog_publisher_release(&f.publisher);
}

/* ---- What the C row adds to the Rust row's tests ---- */

/* The loss of one reader is the sum of its lags, not the last one. */
static void a_readers_loss_is_the_sum_of_its_lags(void) {
    fixture_t f;
    entry_t out;
    sce_blog_reader_t reader;
    sce_blog_read_t result;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 5u);
    reader = sce_blog_reader_at(&f.log, 0u);
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_LAGGED && result.skipped == 3u, "the first lag skips 3");
    CHECK(reader.cursor == 3u && reader.lost == 3u, "the reader holds 3 and has lost 3");
    {
        uint64_t n;
        for (n = 5u; n < 9u; n++) {
            const entry_t e = entry_of(n);
            CHECK(sce_blog_publish(&f.publisher, e.words) == n, "publish %llu", (unsigned long long)n);
        }
    }
    /* h = 9: the window is [7, 9), and the reader holds 3. */
    result = read_into(&f, &reader, &out);
    CHECK(result.status == SCE_BLOG_LAGGED && result.skipped == 4u, "the second lag skips 4");
    CHECK(reader.cursor == 7u && reader.lost == 7u, "the loss is the sum, 7");
    sce_blog_publisher_release(&f.publisher);
}

/* A log retains at least one entry and an entry holds at least one word. */
static void the_constructor_refuses_a_shape_it_cannot_run(void) {
    uint64_t storage[SCE_BLOG_STORAGE_WORDS(2u, 2u)];
    sce_blog_t log;
    CHECK(!sce_blog_init(&log, storage, 1u, 2u), "one slot would retain nothing");
    CHECK(!sce_blog_init(&log, storage, 0u, 2u), "no slot would retain nothing");
    CHECK(!sce_blog_init(&log, storage, 2u, 0u), "an entry of no word carries nothing");
    CHECK(!sce_blog_init(&log, NULL, 2u, 2u), "a log needs storage");
    CHECK(sce_blog_init(&log, storage, 2u, 2u), "two slots retain one entry");
    CHECK(sce_blog_retention(&log) == 1u, "and the retention is one");
}

/* Two slots retain one entry: each publish overwrites the only other slot. */
static void a_log_of_two_slots_retains_one_entry(void) {
    uint64_t storage[SCE_BLOG_STORAGE_WORDS(2u, WORDS)];
    sce_blog_t log;
    sce_blog_publisher_t publisher;
    sce_blog_reader_t reader;
    sce_blog_read_t result;
    uint64_t out[WORDS];
    uint64_t n;
    CHECK(sce_blog_init(&log, storage, 2u, WORDS), "two slots");
    CHECK(sce_blog_publisher_acquire(&log, &publisher), "the publisher");
    for (n = 0; n < 7u; n++) {
        const entry_t e = entry_of(n);
        (void)sce_blog_publish(&publisher, e.words);
    }
    reader = sce_blog_reader_at(&log, 0u);
    result = sce_blog_read(&log, &reader, out);
    CHECK(result.status == SCE_BLOG_LAGGED && result.skipped == 6u, "of 7 entries one is retained, 6 are behind");
    result = sce_blog_read(&log, &reader, out);
    CHECK(result.status == SCE_BLOG_ITEM && result.seq == 6u && is_whole_entry(out, 6u), "the retained one is entry 6");
    sce_blog_publisher_release(&publisher);
}

/* A log that has published its horizon of entries publishes nothing more, and
 * says so; the head stays where it was. */
static void a_log_at_its_horizon_publishes_nothing_more(void) {
    fixture_t f;
    entry_t e;
    fixture_open(&f);
    CHECK(sce_blog_publisher_acquire(&f.log, &f.publisher), "the publisher");
    publish_through(&f, 1u);
    f.publisher.next = SCE_BLOG_HORIZON;
    e = entry_of(0u);
    CHECK(sce_blog_publish(&f.publisher, e.words) == SCE_BLOG_NO_SEQUENCE,
          "past the horizon there is no sequence number");
    CHECK(sce_blog_head(&f.log) == 1u, "and nothing was published");
    CHECK(f.publisher.next == SCE_BLOG_HORIZON, "the publisher's count did not move");
    sce_blog_publisher_release(&f.publisher);
}

/* The stamps are a function of the sequence number alone. */
static void the_stamps_are_busy_and_done_of_the_sequence_number(void) {
    CHECK(sce_blog_busy(0u) == 1u && sce_blog_done(0u) == 2u, "entry 0");
    CHECK(sce_blog_busy(5u) == 11u && sce_blog_done(5u) == 12u, "entry 5");
    CHECK(sce_blog_done(SCE_BLOG_HORIZON - 1u) < UINT64_MAX, "the last stamp fits");
}

/* ---- Real threads ---- */

#define STRESS_ENTRIES 200000u
#define STRESS_READERS 3u
#define STRESS_MOST_SLOTS 9u
#define DEADLINE_SPINS 400000000ull

typedef struct stress {
    sce_blog_t log;
    uint64_t total;
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
    sce_blog_publisher_t publisher;
    uint64_t n;
    if (!sce_blog_publisher_acquire(&s->log, &publisher)) {
        s->failed = 1u;
        return NULL;
    }
    for (n = 0; n < s->total; n++) {
        const entry_t e = entry_of(n);
        if (sce_blog_publish(&publisher, e.words) != n) {
            s->failed = 1u;
            break;
        }
        /* Now and then give the readers the processor, so that they are
         * sometimes ahead of the publisher and sometimes far behind it. */
        if (n % 1024u == 1023u) {
            (void)sched_yield();
        }
    }
    sce_blog_publisher_release(&publisher);
    return NULL;
}

static void *stress_read(void *arg) {
    stress_reader_t *r = (stress_reader_t *)arg;
    stress_t *s = r->stress;
    sce_blog_reader_t reader = sce_blog_reader_at(&s->log, 0u);
    uint64_t last_item_plus_one = 0;
    uint64_t spins = 0;
    while (reader.cursor < s->total) {
        uint64_t out[WORDS];
        const uint64_t before = reader.cursor;
        const uint64_t lost_before = reader.lost;
        const sce_blog_read_t result = sce_blog_read(&s->log, &reader, out);
        if (result.status == SCE_BLOG_ITEM) {
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
        } else if (result.status == SCE_BLOG_LAGGED) {
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
    static uint64_t pool[SCE_BLOG_STORAGE_WORDS(STRESS_MOST_SLOTS, WORDS)];
    stress_t s;
    stress_reader_t readers[STRESS_READERS];
    unsigned i;
    memset(&s, 0, sizeof s);
    memset(readers, 0, sizeof readers);
    CHECK(slots <= STRESS_MOST_SLOTS, "the pool holds %u slots", (unsigned)STRESS_MOST_SLOTS);
    CHECK(sce_blog_init(&s.log, pool, slots, WORDS), "a log of %u slots", (unsigned)slots);
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
    CHECK(sce_blog_head(&s.log) == STRESS_ENTRIES, "every entry was published");
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
    one_publisher_and_several_readers_see_whole_entries(3u);
    one_publisher_and_several_readers_see_whole_entries(9u);
}

int main(void) {
    an_empty_log_is_empty_to_every_reader();
    entries_come_back_in_order_with_their_words();
    the_window_is_exactly_the_retention();
    readers_do_not_share_a_cursor_or_a_loss();
    a_reader_at_the_head_sees_only_what_comes_after();
    there_is_one_publisher_and_its_successor_continues_the_sequence();
    publishing_past_the_window_keeps_the_newest_entries();
    a_readers_loss_is_the_sum_of_its_lags();
    the_constructor_refuses_a_shape_it_cannot_run();
    a_log_of_two_slots_retains_one_entry();
    a_log_at_its_horizon_publishes_nothing_more();
    the_stamps_are_busy_and_done_of_the_sequence_number();
    real_threads_see_whole_entries_and_lose_only_what_they_are_told();
    (void)printf("broadcast log runtime (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
