/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: load_acquire store_release acquire_release all */

/*
 * What this harness is expected to catch, and what has not been measured. The
 * four mutants listed above weaken the hosted atomics' acquire loads alone, the
 * release stores alone, the pair, and every ordering. Each leaves the reader's
 * relaxed copy of the words unordered against the publisher's relaxed writes of
 * them, so a reader can hand over a slot whose stamp reads `done` and whose words
 * are the zeros the log started with or the words of a later entry; the harness
 * judges every item whole and so reports it. They were chosen by reading the
 * protocol and were NOT run on GenMC when this was written. The mutants that
 * weaken the two fences (`fence_acquire`, `fence_release`) are not listed as
 * required for the same reason: the gate prints each one that survives, and one
 * that does not survive is promoted to the list by whoever reads that output.
 */

/*
 * Layer 3 of the broadcast-log kind's verification (ADR 0062 section 6): the C11
 * bounded log (sce/forge/broadcast_log.h) under GenMC, with a slot that is
 * reused. It is the C counterpart of the loom model
 * `a_reader_the_publisher_laps_is_told_how_far`
 * (backends/rust/forge-runtime/tests/loom_broadcast_log.rs).
 *
 * A log of two slots retains one entry. The publisher publishes three, so the
 * slot of entry 0 is written again for entry 2, and a reader that starts at
 * entry 0 is either handed it whole, told it is gone, or finds the log empty.
 * The reader reads three times, so it also crosses the lag.
 *
 * Every answer is held to the specification of ADR 0062 section 2: an item is the
 * whole entry at the cursor (every word is its sequence number plus one, so a
 * mix of two entries and a slot nothing was written to both show), the cursor
 * moves by exactly what the answer says, and a lag skips at least one entry.
 *
 * See scq64_two_threads.c for why the harness includes the hosted atomics whole.
 * The runtime copies its words in a loop and not with memcpy, so no copy is
 * hidden from GenMC here.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>

#include "../conformance/sce_atomic_host.c"

#include <sce/forge/broadcast_log.h>

#include <genmc.h>

#define SLOTS 2u
#define WORDS 2u
#define PUBLISHES 3u

static sce_blog_t g_log;
static uint64_t g_storage[SCE_BLOG_STORAGE_WORDS(SLOTS, WORDS)];

static void *publish_three(void *arg) {
    sce_blog_publisher_t publisher;
    uint64_t n;
    (void)arg;
    if (!sce_blog_publisher_acquire(&g_log, &publisher)) {
        return NULL;
    }
    for (n = 0; n < PUBLISHES; n++) {
        uint64_t entry[WORDS];
        uint64_t seq;
        entry[0] = n + 1u;
        entry[1] = n + 1u;
        seq = sce_blog_publish(&publisher, entry);
        assert(seq == n);
        (void)seq;
    }
    sce_blog_publisher_release(&publisher);
    return NULL;
}

/* One read, held to the specification. */
static void read_and_check(sce_blog_reader_t *reader) {
    uint64_t out[WORDS];
    const uint64_t before = reader->cursor;
    const uint64_t lost_before = reader->lost;
    sce_blog_read_t result;
    out[0] = UINT64_MAX;
    out[1] = UINT64_MAX;
    result = sce_blog_read(&g_log, reader, out);
    if (result.status == SCE_BLOG_ITEM) {
        assert(result.seq == before);
        assert(out[0] == before + 1u);
        assert(out[1] == before + 1u);
        assert(reader->cursor == before + 1u);
        assert(reader->lost == lost_before);
    } else if (result.status == SCE_BLOG_EMPTY) {
        assert(reader->cursor == before);
    } else {
        assert(result.skipped >= 1u);
        assert(reader->cursor == before + result.skipped);
        assert(reader->lost == lost_before + result.skipped);
        assert(reader->cursor <= PUBLISHES);
    }
}

static void *read_three(void *arg) {
    sce_blog_reader_t reader;
    unsigned i;
    (void)arg;
    reader = sce_blog_reader_at(&g_log, 0u);
    for (i = 0; i < 3u; i++) {
        read_and_check(&reader);
    }
    return NULL;
}

int main(void) {
    pthread_t publisher;
    pthread_t reader;

    if (!sce_blog_init(&g_log, g_storage, SLOTS, WORDS)) {
        return 1;
    }
    pthread_create(&publisher, NULL, publish_three, NULL);
    pthread_create(&reader, NULL, read_three, NULL);
    pthread_join(publisher, NULL);
    pthread_join(reader, NULL);

    /* Everything is published: the window is the last entry, and a reader at the
     * oldest finds exactly it and then the head. */
    {
        sce_blog_reader_t late = sce_blog_reader_oldest(&g_log);
        uint64_t out[WORDS];
        sce_blog_read_t result = sce_blog_read(&g_log, &late, out);
        assert(result.status == SCE_BLOG_ITEM && result.seq == PUBLISHES - 1u);
        assert(out[0] == PUBLISHES && out[1] == PUBLISHES);
        result = sce_blog_read(&g_log, &late, out);
        assert(result.status == SCE_BLOG_EMPTY);
    }
    return 0;
}
