/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: all */

/*
 * What this harness catches, measured 2026-10-10 on GenMC v0.19.0 (RC11): only
 * every ordering weakened at once (`all`). Each ordering weakened alone, and the
 * groups (the threshold's seq_cst load and store, the read-modify-writes, the
 * acquire/release pair), survive: one push and one pop never reuse a slot, and
 * with the slot fresh the orderings are redundant with one another.
 * scq64_slot_reuse.c is the harness in which the compare-and-swap stands alone.
 * `all` being caught is what shows the harness reads the orderings at all.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 SCQ queue under GenMC, which explores every execution
 * the C11 memory model allows rather than the few a machine happens to give.
 *
 * A producer pushes one element whose two fields it writes separately, and a
 * consumer pops it, each on a thread of its own. The element is the point: the
 * queue hands over indices, and the element's bytes travel by the ordering of
 * the hand-over alone, so a hand-over that is too weak lets the consumer read
 * the slot before the producer's write is visible. The assertion is that
 * whatever the consumer is given is exactly what was written: both fields, the
 * same push.
 *
 * The consumer pops until it is given an element; GenMC bounds the loop
 * (`--unroll`), and an execution in which the producer has not yet run is cut
 * off by `__VERIFIER_assume`, never read as a pass. What it explores is
 * therefore only executions in which the element was delivered.
 *
 * The atomics are the runtime's own `sce_atomic_*` symbols, supplied by the
 * hosted implementation included whole below, so the checker judges the
 * orderings the runtime asks for.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

/*
 * ⚠ The queue moves an element with `memcpy(slot, value, element_size)`, and
 * `element_size` reaches it as a run-time value. GenMC cannot turn a
 * variable-length `memcpy` into memory events ("Cannot promote non-constant-
 * length mem intrinsic! Skipping"), so it does not see the slot being written
 * or read at all, and a hand-over ordering weakened until the consumer reads
 * the slot early has no read to flag. Measured 2026-10-10: with every
 * read-modify-write of the runtime weakened to relaxed this harness still
 * reported no error, while a plain message-passing probe with the same
 * weakening was caught at once.
 *
 * So the copy is written out here, a word at a time, as the plain loads and
 * stores GenMC does follow. `element_t` is made of 64-bit words, which keeps
 * the copy exact. This is the one place the harness differs from the runtime
 * and it differs in how bytes move, not in any ordering.
 */
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

/* GenMC's assumption primitive: an execution that violates it is not explored
 * further and is not a counterexample. The header comes with the install
 * (`<prefix>/include/genmc/runtime`), which the gate passes as an include path. */
#include <genmc.h>

#define CAPACITY 2u
#define RING_SLOTS 2u

/* Two fields written one after the other. A torn read shows as a pair that were
 * not written together. */
typedef struct {
    uint64_t a;
    uint64_t b;
} element_t;

static sce_queue_scq64_t g_queue;
static uint64_t g_allocated_entries[2u * RING_SLOTS];
static uint64_t g_free_entries[2u * RING_SLOTS];
static element_t g_slots[CAPACITY];

static element_t g_got;
static int g_delivered;

static void *produce(void *arg) {
    element_t e;
    (void)arg;
    e.a = 7u;
    e.b = 7u;
    (void)sce_queue_scq64_try_push(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e);
    return NULL;
}

static void *consume(void *arg) {
    element_t e;
    (void)arg;
    if (sce_queue_scq64_try_pop(&g_queue, (unsigned char *)g_slots, sizeof(element_t), &e)) {
        g_got = e;
        g_delivered = 1;
    }
    return NULL;
}

int main(void) {
    pthread_t producer;
    pthread_t consumer;

    if (!sce_queue_scq64_init(&g_queue, CAPACITY, RING_SLOTS, g_allocated_entries, g_free_entries)) {
        return 1;
    }
    pthread_create(&producer, NULL, produce, NULL);
    pthread_create(&consumer, NULL, consume, NULL);
    pthread_join(producer, NULL);
    pthread_join(consumer, NULL);

    /* Only executions in which the consumer was handed the element say anything
     * about what a hand-over carries. */
    __VERIFIER_assume(g_delivered);
    assert(g_got.a == 7u && g_got.b == 7u);
    return 0;
}
