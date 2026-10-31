/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: load_acquire store_release acquire_release all */

/*
 * What this harness is expected to catch, and what has not been measured. The four
 * mutants listed above weaken the hosted atomics' acquire loads alone, the release
 * stores alone, the pair, and every ordering, as broadcast_log_slot_reuse.c's do, for
 * the same reason: each leaves the reader's relaxed copy of the words unordered against
 * the publisher's relaxed writes of them, so a reader can hand over a slot whose stamp
 * reads `done` and whose words are the zeros the log started with, the words of an
 * earlier entry or the words of a later one; the harness judges every item whole by the
 * full count of its entry, and so reports it. They were chosen by reading the protocol
 * and were NOT run on GenMC when this was written; the mutants that weaken the two
 * fences (`fence_acquire`, `fence_release`) are left unlisted, and the gate prints each
 * one that survives. The u32 atomics the narrow row uses are the same macro as the u64
 * ones in the hosted atomics, so each mutant weakens both widths alike.
 */

/*
 * Layer 3 of the broadcast-log kind's verification (ADR 0062 section 6), for the narrow
 * row: the C11 narrow log (sce/forge/broadcast_log_narrow.h) under GenMC, with a slot
 * that is reused after the stamp has wrapped. It is the C counterpart of the loom models
 * of the Rust lane `Narrow<3>`
 * (backends/rust/forge-runtime/tests/loom_broadcast_log_narrow.rs) and of
 * broadcast_log_slot_reuse.c.
 *
 * The stamp is 3 bits, so the stamp space is 8 values, half of it 4, and the head and
 * the stamps wrap after eight publishes; the log has two slots and retains one entry.
 * The main thread first publishes six entries, 0 to 5, which GenMC explores as one
 * execution, and opens a reader at the head, the count 6. Then the publisher publishes
 * three more, 6, 7 and 8, so the head wraps from 7 to 0 and the slot of entry 0 is
 * written again for entry 8, whose stamp is entry 0's, while the reader reads three
 * times. The premise of the wrap bound holds by construction: at most three entries are
 * begun after the reader's cursor was set, and a stamp tells four apart.
 *
 * The reader keeps the full count its cursor stands for beside the cursor, which is the
 * count modulo 8, and every answer is held to the specification of ADR 0062 section 2
 * by that count: an item is the whole entry at the cursor (every word is the entry's
 * full count plus one, so a mix of two entries, a slot nothing was written to and the
 * entry of a lap on, whose stamp is the same, all show), the cursor moves by exactly what
 * the answer says, and a lag skips at least one entry and lands on the oldest retained
 * entry of a head that is at least 8 and at most 9.
 *
 * See scq64_two_threads.c for why the harness includes the hosted atomics whole.
 * The runtime copies its words in a loop and not with memcpy, so no copy is hidden from
 * GenMC here.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>

#include "../conformance/sce_atomic_host.c"

#define SCE_BLOGN_STAMP_BITS 3u
#include <sce/forge/broadcast_log_narrow.h>

#include <genmc.h>

#define SLOTS 2u
#define WORDS 2u
#define PRIMED 6u
#define PUBLISHES 3u

static sce_blogn_t g_log;
static uint32_t g_storage[SCE_BLOGN_STORAGE_WORDS(SLOTS, WORDS)];
static sce_blogn_reader_t g_reader;

static void *publish_three(void *arg) {
    sce_blogn_publisher_t publisher = {0};
    uint32_t n;
    (void)arg;
    if (!sce_blogn_publisher_acquire(&g_log, &publisher)) {
        return NULL;
    }
    for (n = PRIMED; n < PRIMED + PUBLISHES; n++) {
        uint32_t entry[WORDS];
        uint32_t seq;
        entry[0] = n + 1u;
        entry[1] = n + 1u;
        seq = sce_blogn_publish(&publisher, entry);
        assert(seq == (n & SCE_BLOGN_MASK));
        (void)seq;
    }
    sce_blogn_publisher_release(&publisher);
    return NULL;
}

/* One read, held to the specification by the full count `*full` of the reader's cursor. */
static void read_and_check(sce_blogn_reader_t *reader, uint64_t *full) {
    uint32_t out[WORDS];
    const uint32_t before = reader->cursor;
    const uint64_t lost_before = reader->lost;
    sce_blogn_read_t result;
    assert(before == (uint32_t)(*full & SCE_BLOGN_MASK));
    out[0] = UINT32_MAX;
    out[1] = UINT32_MAX;
    result = sce_blogn_read(&g_log, reader, out);
    if (result.status == SCE_BLOGN_ITEM) {
        assert(result.seq == before);
        assert(out[0] == (uint32_t)(*full + 1u));
        assert(out[1] == (uint32_t)(*full + 1u));
        *full += 1u;
        assert(reader->cursor == (uint32_t)(*full & SCE_BLOGN_MASK));
        assert(reader->lost == lost_before);
    } else if (result.status == SCE_BLOGN_EMPTY) {
        assert(reader->cursor == before);
    } else {
        /* The oldest retained entry of a head h is h - 1, and a lag is answered only once
         * the head has reached the entry after the next slot's lap, 8, and never passes 9. */
        assert(result.skipped >= 1u);
        *full += result.skipped;
        assert(reader->cursor == (uint32_t)(*full & SCE_BLOGN_MASK));
        assert(reader->lost == lost_before + result.skipped);
        assert(*full >= 7u && *full <= 8u);
    }
}

static void *read_three(void *arg) {
    sce_blogn_reader_t reader = g_reader;
    uint64_t full = PRIMED;
    unsigned i;
    (void)arg;
    for (i = 0; i < 3u; i++) {
        read_and_check(&reader, &full);
    }
    return NULL;
}

int main(void) {
    pthread_t publisher;
    pthread_t reader;
    sce_blogn_publisher_t primer = {0};
    uint32_t n;

    if (!sce_blogn_init(&g_log, g_storage, SLOTS, WORDS)) {
        return 1;
    }
    if (!sce_blogn_split(&g_log, &primer)) {
        return 1;
    }
    for (n = 0; n < PRIMED; n++) {
        uint32_t entry[WORDS];
        entry[0] = n + 1u;
        entry[1] = n + 1u;
        (void)sce_blogn_publish(&primer, entry);
    }
    sce_blogn_publisher_release(&primer);
    g_reader = sce_blogn_reader_head(&g_log);
    assert(g_reader.cursor == PRIMED);

    pthread_create(&publisher, NULL, publish_three, NULL);
    pthread_create(&reader, NULL, read_three, NULL);
    pthread_join(publisher, NULL);
    pthread_join(reader, NULL);

    /* Everything is published, nine entries: the head is 9 modulo 8 = 1 and the window is the
     * last entry, count 8, residue 0. A reader at the oldest finds exactly it, by a stamp
     * that is the one of entry 0, and then the head. */
    {
        sce_blogn_reader_t late = sce_blogn_reader_oldest(&g_log);
        uint32_t out[WORDS];
        sce_blogn_read_t result;
        assert(late.cursor == 0u);
        result = sce_blogn_read(&g_log, &late, out);
        assert(result.status == SCE_BLOGN_ITEM && result.seq == 0u);
        assert(out[0] == PRIMED + PUBLISHES && out[1] == PRIMED + PUBLISHES);
        result = sce_blogn_read(&g_log, &late, out);
        assert(result.status == SCE_BLOGN_EMPTY);
    }
    return 0;
}
