/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: cas rmw all */

/*
 * What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
 * baseline explores 616 complete executions with no error, and the
 * compare-and-swap orderings weakened (`cas`), every read-modify-write ordering
 * weakened (`rmw`) and every ordering weakened (`all`) are each reported as a
 * non-atomic race. It does NOT catch the acquire loads, the release stores, the
 * pair, or the sequentially consistent accesses weakened, alone or together, and
 * it does not catch the fence weakened alone or every sequentially consistent
 * ordering weakened with it (the last two run by hand): in this scenario the
 * compare-and-swaps provide the hand-overs those would, and nothing here depends
 * on the hazard handshake. They are listed as surviving rather than required,
 * because requiring them would be a claim this harness cannot back.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 LSCQ (the `segmented` row for any cardinality but one
 * producer and one consumer) under GenMC, across a segment boundary with a
 * retirement.
 *
 * The main thread pushes two elements before the threads start. The segment holds
 * one element, so the second push closes the first segment and links a second
 * behind it holding the element. Then one thread pushes a third element, into the
 * second segment, while the other pops: it takes the first element, finds the
 * first segment closed and empty, moves `head` past it, retires it and runs the
 * scan that frees what no hazard names.
 *
 * What it does not exercise is the hazard handshake, which is why the fence and
 * the sequentially consistent orderings survive it. The producer is never inside
 * the segment the consumer retires: the main thread's second push has already
 * moved `tail` to the second segment, and with one producer the only thread that
 * can link a segment is the thread that then leaves it. A producer inside a
 * segment another participant retires needs a second producer, and that scenario,
 * two producers and a consumer, did not finish under GenMC (a bound of ten
 * minutes on the build machine, 312,819 graphs explored, measured 2026-10-11).
 * The handshake is checked on the domain by itself, in
 * hazard_keeps_a_named_node.c.
 *
 * Every element the consumer is given must be exactly one of the three written,
 * whole, and in the order written.
 *
 * The harness supplies `memcpy` itself, for the reason linked_segment_handover.c
 * gives, and includes the hosted atomics whole, for the reason scq64_two_threads.c
 * gives.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

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

/* A dequeue that finds an entry empty looks again this many times before it
 * makes the entry unusable (10000 by default). GenMC explores every iteration of
 * a loop up to its unroll bound and then kills the execution, so the retirement
 * of a segment, which walks the closed ring's empty entries, never finished with
 * the default: the main thread was killed before it created a thread, and the
 * run that reported no error described a program with no concurrency in it
 * (measured 2026-10-11). One look models both outcomes of the loop, an entry
 * that is filled meanwhile and one that is not. */
#define SCE_QUEUE_SCQ_SPINS 1u

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include <genmc.h>

#define SEGMENT_LEN 1u
#define RING_SLOTS 2u
#define HAZARD_SLOTS 2u

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

static sce_queue_lscq_t g_queue;
static sce_queue_allocator_t g_allocator;
static sce_hazard_domain_t g_domain;
static sce_hazard_slot_t g_hazards[HAZARD_SLOTS];

static element_t g_got[3];
static unsigned g_got_count;

static void *produce(void *arg) {
    uint32_t hazard;
    element_t e;
    (void)arg;
    if (!sce_queue_lscq_producer_acquire(&g_queue, &hazard)) {
        return NULL;
    }
    e.a = 3u;
    e.b = 3u;
    (void)sce_queue_lscq_try_push(&g_queue, hazard, &e);
    sce_queue_lscq_producer_release(&g_queue, hazard);
    return NULL;
}

static void *consume(void *arg) {
    uint32_t hazard;
    unsigned i;
    (void)arg;
    if (!sce_queue_lscq_consumer_acquire(&g_queue, &hazard)) {
        return NULL;
    }
    for (i = 0; i < 2u; i++) {
        element_t e;
        if (sce_queue_lscq_try_pop(&g_queue, hazard, &e)) {
            g_got[g_got_count++] = e;
        }
    }
    sce_hazard_collect(&g_domain);
    sce_queue_lscq_consumer_release(&g_queue, hazard);
    return NULL;
}

int main(void) {
    pthread_t producer;
    pthread_t consumer;
    uint32_t hazard;
    unsigned i;

    g_allocator.context = NULL;
    g_allocator.allocate = harness_allocate;
    g_allocator.deallocate = harness_deallocate;
    g_allocator.progress = SCE_QUEUE_PROGRESS_BLOCKING;
    if (!sce_hazard_domain_init(&g_domain, g_hazards, HAZARD_SLOTS)) {
        return 1;
    }
    if (!sce_queue_lscq_init(&g_queue, &g_domain, &g_allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(element_t),
                             sizeof(uint64_t), SEGMENT_LEN, RING_SLOTS)) {
        return 1;
    }

    /* The first two elements, pushed before the threads exist: the second closes
     * the first segment and links the second. */
    if (!sce_queue_lscq_producer_acquire(&g_queue, &hazard)) {
        return 1;
    }
    for (i = 1; i <= 2u; i++) {
        element_t e;
        e.a = i;
        e.b = i;
        (void)sce_queue_lscq_try_push(&g_queue, hazard, &e);
    }
    sce_queue_lscq_producer_release(&g_queue, hazard);

    pthread_create(&producer, NULL, produce, NULL);
    pthread_create(&consumer, NULL, consume, NULL);
    pthread_join(producer, NULL);
    pthread_join(consumer, NULL);

    /* Whole elements, in the order written. */
    for (i = 0; i < g_got_count; i++) {
        assert(g_got[i].a == g_got[i].b);
        assert(g_got[i].a >= 1u && g_got[i].a <= 3u);
        if (i > 0) {
            assert(g_got[i - 1u].a < g_got[i].a);
        }
    }
    /* Every segment still held goes back, and the retired ones with them. */
    sce_queue_lscq_destroy(&g_queue);
    return 0;
}
