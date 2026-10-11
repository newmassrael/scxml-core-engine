/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/* THREADS: 2 */
/* REQUIRED: load_seq_cst store_release threshold acquire_release fence all */

/*
 * What this harness catches, measured 2026-10-11 on GenMC v0.19.0 (RC11): the
 * baseline explores 9 complete executions with no error, and the sequentially
 * consistent re-check load weakened alone (`load_seq_cst`), the release stores
 * weakened alone (`store_release`, which includes the one that gives a slot
 * back), the sequentially consistent loads and stores together (`threshold`),
 * the acquire/release pair (`acquire_release`), the fence weakened alone
 * (`fence`) and every ordering (`all`) are each reported as an attempt to access
 * freed memory. Also caught when run by hand, and not in the gate: every
 * sequentially consistent ordering together with the fence, and the sequentially
 * consistent loads and stores together with the fence kept. The sequentially
 * consistent publication store weakened alone (`store_seq_cst`), the acquire
 * loads weakened alone (`load_acquire`) and the compare-and-swap and
 * fetch-and-modify orderings survived this harness; none is listed as required,
 * and the gate prints each one that survives.
 */

/*
 * Layer 3 of the queue kind's verification (SCE Protocol-Synthesis RFC
 * §synth-5-P): the C11 hazard-pointer domain of the segmented queues under GenMC.
 *
 * The domain's contract is that a node a participant has protected is not freed
 * while it is named, whatever the other participants do. It rests on one
 * handshake: a participant publishes the node it read with a sequentially
 * consistent store, puts a sequentially consistent fence after it and re-reads
 * the location with a sequentially consistent load; whoever unlinks a node puts a
 * sequentially consistent fence after the unlinking compare-and-swap and before
 * the retirement, and the scan reads the hazards after that. Either the scan sees
 * the hazard and keeps the node, or the participant's re-read sees the location
 * already moved and does not use the node.
 *
 * The harness is the smallest program that has both sides. A location holds a
 * node. One thread protects it, reads the node and gives the slot back. The other
 * installs a second node, puts the fence the contract gives to the unlinking side,
 * retires the first and runs the scan. A node read after it was freed is a failure
 * GenMC reports, and the contract is what keeps it from happening.
 *
 * It is not the LSCQ. The whole LSCQ with two producers and a consumer, the
 * scenario in which a producer can be inside a segment another participant
 * retires, did not finish under GenMC (a bound of ten minutes on the build
 * machine, with 312,819 graphs explored, measured 2026-10-11), and with one
 * producer the race cannot occur: the only thread that links a segment is the
 * thread that then leaves it. The part of the LSCQ that holds a segment up, which
 * is this domain and the order in which the consumer takes `tail` and `head`
 * past it, is covered separately: the first here, the second by the unit test
 * `a_consumer_takes_tail_off_a_segment_before_it_retires_it` of the Rust runtime.
 */

#include <assert.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "../conformance/sce_atomic_host.c"

#include <sce/forge/queue.h>
#include <sce/forge/queue_segmented.h>

#include <genmc.h>

#define HAZARD_SLOTS 2u

/* A node the domain can retire: the header first, as the domain requires. */
typedef struct {
    sce_hazard_retired_t retired;
    uint64_t data;
} node_t;

static sce_hazard_domain_t g_domain;
static sce_hazard_slot_t g_hazards[HAZARD_SLOTS];

/* The location the nodes are installed in, as a pointer word. */
static size_t g_source;

static node_t *g_first;
static node_t *g_second;

static void reclaim(sce_hazard_retired_t *node, void *context) {
    (void)context;
    free(node);
}

static void *read_the_node(void *arg) {
    int slot;
    size_t word;
    node_t *node;
    (void)arg;
    slot = sce_hazard_acquire(&g_domain);
    if (slot < 0) {
        return NULL;
    }
    word = sce_hazard_protect(&g_domain, (uint32_t)slot, &g_source);
    node = (node_t *)sce_queue_pointer_of(word);
    /* Whichever node was read, it is whole and not yet freed. */
    assert(node->data == 1u || node->data == 2u);
    sce_hazard_release(&g_domain, (uint32_t)slot);
    return NULL;
}

static void *replace_the_node(void *arg) {
    size_t seen;
    (void)arg;
    seen = sce_atomic_cas_strong_acq_rel_usize(&g_source, sce_queue_word_of(g_first), sce_queue_word_of(g_second));
    if (seen == sce_queue_word_of(g_first)) {
        /* The fence the unlinking side owes the contract, then the retirement
         * and a scan, which frees the node if no hazard names it. */
        sce_atomic_fence_seq_cst();
        sce_hazard_retire(&g_domain, &g_first->retired, reclaim, NULL);
        sce_hazard_collect(&g_domain);
    }
    return NULL;
}

int main(void) {
    pthread_t reader;
    pthread_t replacer;

    if (!sce_hazard_domain_init(&g_domain, g_hazards, HAZARD_SLOTS)) {
        return 1;
    }
    g_first = (node_t *)malloc(sizeof(node_t));
    g_second = (node_t *)malloc(sizeof(node_t));
    g_first->retired.chain = 0u;
    g_first->retired.reclaim = NULL;
    g_first->retired.context = NULL;
    g_first->data = 1u;
    g_second->retired.chain = 0u;
    g_second->retired.reclaim = NULL;
    g_second->retired.context = NULL;
    g_second->data = 2u;
    g_source = sce_queue_word_of(g_first);

    pthread_create(&reader, NULL, read_the_node, NULL);
    pthread_create(&replacer, NULL, replace_the_node, NULL);
    pthread_join(reader, NULL);
    pthread_join(replacer, NULL);

    /* What the scan kept and what is still installed go back. */
    sce_hazard_flush(&g_domain);
    free(g_second);
    return 0;
}
