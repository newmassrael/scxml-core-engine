/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: load_acquire store_release acquire_release all */

/*
 * What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
 * baseline explores 11 complete executions with no error, and the acquire loads
 * weakened alone (`load_acquire`), the release stores weakened alone
 * (`store_release`), the pair (`acquire_release`) and every ordering (`all`) are
 * each reported as an attempt to read from uninitialized memory: the consumer
 * reads a segment or a slot whose write the producer's hand-over no longer
 * orders before it. The other mutants of the gate survived this harness; they
 * weaken orderings the ring does not use for a hand-over (a sequentially
 * consistent access, a compare-and-swap, a fetch), so none is listed as
 * required, and the gate prints each one that survives.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 linked Lamport rings (the `segmented` row for one producer
 * and one consumer) under GenMC, across a segment boundary.
 *
 * The segment holds one element, so the producer's second push takes a segment
 * from the allocator, links it behind the first and puts the element in it. Two
 * kinds of hand-over carry the elements, each one release store read by one
 * acquire load: the count of filled slots, which publishes an element to the
 * consumer, and the link, which publishes the successor segment. The consumer
 * also gives the first segment back to the allocator once it has read the link.
 *
 * The allocator is the C library's `malloc` and `free`, which GenMC models: a
 * block is uninitialized until the runtime writes it, so a read the producer's
 * write is not ordered before is an error GenMC reports.
 *
 * Every element the consumer is given must be exactly one of the two written,
 * whole, and in the order written. A pop that finds nothing yet is not a failure;
 * the harness tries three times and judges only what was delivered.
 *
 * The copy is written out and the harness includes the hosted atomics whole, for
 * the reasons scq64_two_threads.c gives. It defines `memcpy` itself, and the
 * comment at the definition says why that is more than the other harnesses do.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

/* The harness supplies `memcpy` itself, written out word by word. The runtime
 * copies an element with `memcpy`, and it copies the allocator it is given with a
 * structure assignment, which the compiler lowers to a call of the same function:
 * GenMC promotes a copy it can see the types of, and this one it cannot (the
 * destination is the first member of the queue, so its address is the queue's),
 * which it reports as `Invalid call to memcpy()` (measured 2026-10-11). A
 * definition here is what that call reaches. */
void *memcpy(void *destination, const void *source, size_t bytes) {
    uint64_t *to = (uint64_t *)destination;
    const uint64_t *from = (const uint64_t *)source;
    size_t words = bytes / sizeof(uint64_t);
    size_t i;
    for (i = 0; i < words; i++) {
        to[i] = from[i];
    }
    return destination;
}

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include <genmc.h>

#define SEGMENT_LEN 1u

typedef struct {
    uint64_t a;
    uint64_t b;
} element_t;

static void *harness_allocate(void *context, size_t size, size_t align) {
    (void)context;
    (void)align;
    return malloc(size);
}

static void harness_deallocate(void *context, void *block, size_t size, size_t align) {
    (void)context;
    (void)size;
    (void)align;
    free(block);
}

static sce_queue_linked_t g_queue;
static sce_queue_allocator_t g_allocator;

static element_t g_got[3];
static unsigned g_got_count;

static void *produce(void *arg) {
    unsigned i;
    (void)arg;
    if (!sce_queue_linked_producer_acquire(&g_queue)) {
        return NULL;
    }
    for (i = 1; i <= 2u; i++) {
        element_t e;
        e.a = i;
        e.b = i;
        (void)sce_queue_linked_try_push(&g_queue, &e);
    }
    sce_queue_linked_producer_release(&g_queue);
    return NULL;
}

static void *consume(void *arg) {
    unsigned i;
    (void)arg;
    if (!sce_queue_linked_consumer_acquire(&g_queue)) {
        return NULL;
    }
    for (i = 0; i < 3u; i++) {
        element_t e;
        if (sce_queue_linked_try_pop(&g_queue, &e)) {
            g_got[g_got_count++] = e;
        }
    }
    sce_queue_linked_consumer_release(&g_queue);
    return NULL;
}

int main(void) {
    pthread_t producer;
    pthread_t consumer;
    unsigned i;

    g_allocator.context = NULL;
    g_allocator.allocate = harness_allocate;
    g_allocator.deallocate = harness_deallocate;
    g_allocator.progress = SCE_QUEUE_PROGRESS_BLOCKING;
    if (!sce_queue_linked_init(&g_queue, &g_allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(element_t), sizeof(uint64_t),
                               SEGMENT_LEN)) {
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
    /* Every segment still held goes back; GenMC reports a segment freed twice or
     * one the consumer already gave back. */
    sce_queue_linked_destroy(&g_queue);
    return 0;
}
