/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: load_acquire store_release acquire_release all */

/*
 * What this harness catches, measured 2026-10-10 on GenMC v0.19.0 (RC11): the
 * acquire loads weakened alone (`load_acquire`), the release stores weakened
 * alone (`store_release`), the pair (`acquire_release`) and every ordering
 * (`all`). Each is a race on the slot, reported by GenMC as a non-atomic race:
 * the ring's only hand-overs are a release store read by an acquire load, so
 * weakening either end leaves the consumer's read and the producer's write of
 * the slot unordered. The remaining mutants of the gate were not run against
 * this harness when this was written; they weaken orderings the ring does not
 * use for a hand-over, so none is listed as required, and the gate prints each
 * one that survives.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 Lamport ring (the `bounded` row for one producer and one
 * consumer) under GenMC, with a slot that is reused.
 *
 * A ring of one slot holds one element at a time. The producer pushes two
 * elements, so the second can only go into the slot the first was popped from.
 * Two hand-overs carry the elements and each is one release store read by one
 * acquire load: the producer's store of `tail` publishes a filled slot to the
 * consumer, and the consumer's store of `head` hands the slot back, so that the
 * consumer's read of the first element happens-before the producer's write of
 * the second. A hand-over that is too weak lets the consumer read a slot the
 * producer is already overwriting, which GenMC reports as a race.
 *
 * Every element the consumer is given must be exactly one of the two written,
 * whole, and in the order written. A full ring refuses the second push and an
 * empty one refuses a pop; the harness tries each once and judges only what was
 * delivered.
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

typedef struct {
    uint64_t a;
    uint64_t b;
} element_t;

static sce_queue_spsc_t g_queue;
static element_t g_slots[CAPACITY];

static element_t g_got[2];
static unsigned g_got_count;

static void *produce(void *arg) {
    unsigned i;
    (void)arg;
    if (!sce_queue_spsc_producer_acquire(&g_queue)) {
        return NULL;
    }
    for (i = 1; i <= 2u; i++) {
        element_t e;
        e.a = i;
        e.b = i;
        /* A full ring refuses the push; the producer tries the next element,
         * so the consumer may be given either, or both. */
        (void)sce_queue_spsc_try_push(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e);
    }
    sce_queue_spsc_producer_release(&g_queue);
    return NULL;
}

static void *consume(void *arg) {
    unsigned i;
    (void)arg;
    if (!sce_queue_spsc_consumer_acquire(&g_queue)) {
        return NULL;
    }
    for (i = 0; i < 2u; i++) {
        element_t e;
        if (sce_queue_spsc_try_pop(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e)) {
            g_got[g_got_count++] = e;
        }
    }
    sce_queue_spsc_consumer_release(&g_queue);
    return NULL;
}

int main(void) {
    pthread_t producer;
    pthread_t consumer;
    unsigned i;

    if (!sce_queue_spsc_init(&g_queue, CAPACITY)) {
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
