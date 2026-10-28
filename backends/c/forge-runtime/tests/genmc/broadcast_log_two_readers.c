/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 3 */
/* REQUIRED: load_acquire store_release acquire_release all */

/*
 * What this harness is expected to catch, and what has not been measured: the same
 * four mutants as broadcast_log_slot_reuse.c, for the same reason, and with the same
 * caveat. They were chosen by reading the protocol and were NOT run on GenMC when
 * this was written; the two fence mutants are left unlisted and the gate prints
 * each one that survives.
 */

/*
 * Layer 3 of the broadcast-log kind's verification (ADR 0062 section 6): the C11
 * bounded log (sce/forge/broadcast_log.h) under GenMC with two readers. It is the C
 * counterpart of the loom model `two_readers_each_hold_their_own_cursor`
 * (backends/rust/forge-runtime/tests/loom_broadcast_log.rs), the model that found
 * that a relaxed busy stamp is a fault: a reader that meets the stamp of a later
 * lap reloads `head` and must find the publish that began that lap, or it reports
 * a lag that skipped nothing.
 *
 * A log of two slots retains one entry; the publisher publishes three, so the
 * slot of entry 0 is written again for entry 2. Two readers, each with its own
 * cursor from the oldest retained entry, read twice while that happens. Every
 * answer is held to the specification of ADR 0062 section 2: an item is the whole
 * entry at the cursor (every word is its sequence number plus one), the cursor
 * moves by exactly what the answer says, a lag skips at least one entry, and one
 * reader's reads move nothing of the other's.
 *
 * See scq64_two_threads.c for why the harness includes the hosted atomics whole.
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

static void *read_twice(void *arg) {
    sce_blog_reader_t reader;
    unsigned i;
    (void)arg;
    reader = sce_blog_reader_oldest(&g_log);
    for (i = 0; i < 2u; i++) {
        read_and_check(&reader);
    }
    return NULL;
}

int main(void) {
    pthread_t publisher;
    pthread_t first;
    pthread_t second;

    if (!sce_blog_init(&g_log, g_storage, SLOTS, WORDS)) {
        return 1;
    }
    pthread_create(&publisher, NULL, publish_three, NULL);
    pthread_create(&first, NULL, read_twice, NULL);
    pthread_create(&second, NULL, read_twice, NULL);
    pthread_join(publisher, NULL);
    pthread_join(first, NULL);
    pthread_join(second, NULL);
    return 0;
}
