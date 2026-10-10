/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: cas rmw all */

/*
 * What this harness catches, measured 2026-10-10 on GenMC v0.19.0 (RC11): the
 * compare-and-swap weakened to relaxed (`cas`), every read-modify-write weakened
 * (`rmw`) and every ordering weakened (`all`). It does NOT catch an acquire
 * load, a seq_cst load or store, a fetch-and-add or a fetch-and-or weakened
 * alone, nor the threshold's load and store together, nor the acquire/release
 * pair: in this scenario another ordering provides the hand-over each of those
 * would, so weakening one leaves the element whole. They are listed as
 * surviving rather than required because requiring them would be a claim this
 * harness cannot back; a scenario that makes one of them the only provider is
 * the way to require it.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 SCQ queue under GenMC, with a slot that is reused.
 *
 * A queue of one slot holds one element at a time. The producer pushes two
 * elements, so the second can only go into the slot the first was popped from:
 * the producer's write of the second element races the consumer's read of the
 * first unless the slot's hand-back (the consumer's push of the index onto the
 * free ring) is ordered before the producer's re-take of it. That ordering is
 * what a single push and pop never exercise (scq64_two_threads.c), and it is
 * the one that protects a slot from being overwritten while it is read.
 *
 * Every element the consumer is given must be exactly one of the two written,
 * whole, and in the order written.
 *
 * See scq64_two_threads.c for why the copy is written out and why the harness
 * includes the hosted atomics whole.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

static void *genmc_copy(void *destination, const void *source, size_t bytes) {
    uint64_t *to = (uint64_t *)destination;
    const uint64_t *from = (const uint64_t *)source;
    size_t words = bytes / sizeof(uint64_t);
    size_t i;
    for (i = 0; i < words; i++) {
        to[i] = from[i];
    }
    return destination;
}

#define memcpy genmc_copy

#include <sce/forge/queue.h>

#include <genmc.h>

#define CAPACITY 1u
#define RING_SLOTS 1u

typedef struct {
    uint64_t a;
    uint64_t b;
} element_t;

static sce_queue_scq64_t g_queue;
static uint64_t g_allocated_entries[2u * RING_SLOTS];
static uint64_t g_free_entries[2u * RING_SLOTS];
static element_t g_slots[CAPACITY];

static element_t g_got[2];
static unsigned g_got_count;

static void *produce(void *arg) {
    unsigned i;
    (void)arg;
    for (i = 1; i <= 2u; i++) {
        element_t e;
        e.a = i;
        e.b = i;
        /* A full queue refuses the push; the producer tries the next element,
         * so the consumer may be given either, or both. */
        (void)sce_queue_scq64_try_push(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e);
    }
    return NULL;
}

static void *consume(void *arg) {
    unsigned i;
    (void)arg;
    for (i = 0; i < 2u; i++) {
        element_t e;
        if (sce_queue_scq64_try_pop(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e)) {
            g_got[g_got_count++] = e;
        }
    }
    return NULL;
}

int main(void) {
    pthread_t producer;
    pthread_t consumer;
    unsigned i;

    if (!sce_queue_scq64_init(&g_queue, CAPACITY, RING_SLOTS, g_allocated_entries, g_free_entries)) {
        return 1;
    }
    pthread_create(&producer, NULL, produce, NULL);
    pthread_create(&consumer, NULL, consume, NULL);
    pthread_join(producer, NULL);
    pthread_join(consumer, NULL);

    /* Whole elements, in the order written. */
    for (i = 0; i < g_got_count; i++) {
        assert(g_got[i].a == g_got[i].b);
        assert(g_got[i].a == 1u || g_got[i].a == 2u);
        if (i > 0) {
            assert(g_got[i - 1u].a < g_got[i].a);
        }
    }
    return 0;
}
