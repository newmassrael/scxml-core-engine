/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The queue kind's generated headers (SCE Protocol-Synthesis RFC §synth-5-P),
 * compiled and used.
 *
 * sce-build's tests read what the generator writes for a queue and look for text
 * in it, which shows the text is what the template says and nothing about whether
 * the names in it are the names the runtime has. Here the header the generator
 * writes for each C11 lowering is compiled into this test over the element
 * header it includes, and the queue it defines is used: a rename in the runtime,
 * or a constant the template computes wrongly, stops the build or fails an
 * assertion. The runtime's own properties are queue_runtime_test.c's; this file
 * holds what the generated header adds.
 *
 * The five headers are the five lowerings a C11 queue has, each generated for a
 * machine of a deploy that states the target's atomics (platform.atomic_rmw_width):
 *
 *   queue_c_spsc       one producer, one consumer          64-bit atomics
 *   queue_c_scq64      many, many                          64-bit atomics
 *   queue_c_scq32      many, many                          32-bit atomics
 *   queue_c_irq_one    one, one                            no atomics
 *   queue_c_irq_many   many, many                          no atomics
 *
 * built by CMakeLists.txt from the documents every other backend compiles, with
 * the progress they declare lowered to `blocking` where the target gives no more.
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "host_irq.h"

#include "queue_c_intrusive.h"
#include "queue_c_intrusive_irq.h"
#include "queue_c_irq_many.h"
#include "queue_c_irq_one.h"
#include "queue_c_scq32.h"
#include "queue_c_scq64.h"
#include "queue_c_segmented.h"
#include "queue_c_segmented_many.h"
#include "queue_c_spsc.h"

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

/* The generated element: the queue's constants and functions are checked against
 * the element header's struct by compiling this file at all. */
static queue_conformance_event_t make_event(unsigned sensor, unsigned value) {
    queue_conformance_event_t e;
    memset(&e, 0, sizeof e);
    e.sensor_id = (uint8_t)sensor;
    e.value = (uint16_t)value;
    return e;
}

/* One queue of any lowering, through the generated functions. */
#define EXERCISE(PREFIX, TYPE, UPPER)                                                                                  \
    static void exercise_##PREFIX(void) {                                                                              \
        static TYPE q;                                                                                                 \
        unsigned i;                                                                                                    \
        CHECK(PREFIX##_init(&q), #PREFIX ": the shape the document states is refused");                                \
        CHECK(PREFIX##_producer_acquire(&q) && PREFIX##_consumer_acquire(&q),                                          \
              #PREFIX ": a fresh queue hands out a place a side");                                                     \
        {                                                                                                              \
            queue_conformance_event_t none;                                                                            \
            CHECK(!PREFIX##_try_pop(&q, &none), #PREFIX ": a new queue is empty");                                     \
        }                                                                                                              \
        for (i = 0; i < UPPER##_CAPACITY; i++) {                                                                       \
            const queue_conformance_event_t e = make_event(1u, i);                                                     \
            CHECK(PREFIX##_try_push(&q, &e) == SCE_QUEUE_PUSH_OK, #PREFIX ": push %u of %u must fit", i,               \
                  (unsigned)UPPER##_CAPACITY);                                                                         \
        }                                                                                                              \
        {                                                                                                              \
            const queue_conformance_event_t e = make_event(9u, 99u);                                                   \
            CHECK(PREFIX##_try_push(&q, &e) == SCE_QUEUE_PUSH_FULL, #PREFIX ": a full queue refuses the push");        \
        }                                                                                                              \
        for (i = 0; i < UPPER##_CAPACITY; i++) {                                                                       \
            queue_conformance_event_t got;                                                                             \
            CHECK(PREFIX##_try_pop(&q, &got) && got.sensor_id == 1u && got.value == i,                                 \
                  #PREFIX ": pop %u: first in, first out", i);                                                         \
        }                                                                                                              \
        {                                                                                                              \
            queue_conformance_event_t none;                                                                            \
            CHECK(!PREFIX##_try_pop(&q, &none), #PREFIX ": drained");                                                  \
        }                                                                                                              \
        PREFIX##_producer_release(&q);                                                                                 \
        PREFIX##_consumer_release(&q);                                                                                 \
    }

EXERCISE(queue_c_spsc, queue_c_spsc_t, QUEUE_C_SPSC)
EXERCISE(queue_c_scq64, queue_c_scq64_t, QUEUE_C_SCQ64)
EXERCISE(queue_c_scq32, queue_c_scq32_t, QUEUE_C_SCQ32)
EXERCISE(queue_c_irq_one, queue_c_irq_one_t, QUEUE_C_IRQ_ONE)
EXERCISE(queue_c_irq_many, queue_c_irq_many_t, QUEUE_C_IRQ_MANY)

static void each_header_states_what_the_document_required_and_what_it_gives(void) {
    /* The Lamport ring: the document asks for wait-free and gets it. */
    CHECK(QUEUE_C_SPSC_CAPACITY == 4u, "the capacity is the document's: four");
    CHECK(strcmp(QUEUE_C_SPSC_DECLARED_PROGRESS, "wait-free") == 0 &&
              strcmp(QUEUE_C_SPSC_PUSH_PROGRESS, "wait-free") == 0 &&
              strcmp(QUEUE_C_SPSC_POP_PROGRESS, "wait-free") == 0,
          "a Lamport ring gives wait-free");
    CHECK(strstr(QUEUE_C_SPSC_ALGORITHM, "Lamport") != NULL, "one and one select the Lamport ring: %s",
          QUEUE_C_SPSC_ALGORITHM);

    /* SCQ over 64-bit and 32-bit entries: the same document, the target decides the
     * ring and the wrap bound. */
    CHECK(QUEUE_C_SCQ64_CAPACITY == 6u && QUEUE_C_SCQ64_PARTICIPANTS == 3u && QUEUE_C_SCQ64_RING_SLOTS == 8u,
          "capacity 6 and 3 participants: the ring is the next power of two at or above both");
    CHECK(QUEUE_C_SCQ32_RING_SLOTS == 8u, "the 32-bit ring is sized the same way");
    CHECK(QUEUE_C_SCQ64_WRAP_BOUND_OPS == SCE_QUEUE_WRAP_BOUND_OPS_64, "2^62 on 64-bit entries");
    CHECK(QUEUE_C_SCQ32_WRAP_BOUND_OPS == SCE_QUEUE_WRAP_BOUND_OPS_32, "2^30 on 32-bit entries");
    CHECK(strcmp(QUEUE_C_SCQ64_PUSH_PROGRESS, "lock-free") == 0 && strcmp(QUEUE_C_SCQ32_POP_PROGRESS, "lock-free") == 0,
          "SCQ gives lock-free on both widths");
    CHECK(sizeof(((queue_c_scq64_t *)0)->allocated_entries[0]) == 8u, "64-bit entries are eight bytes");
    CHECK(sizeof(((queue_c_scq32_t *)0)->allocated_entries[0]) == 4u, "32-bit entries are four bytes");

    /* No atomics: every row is blocking, whatever the table would give, and the
     * document declared exactly that. */
    CHECK(strcmp(QUEUE_C_IRQ_ONE_DECLARED_PROGRESS, "blocking") == 0 &&
              strcmp(QUEUE_C_IRQ_ONE_PUSH_PROGRESS, "blocking") == 0 &&
              strcmp(QUEUE_C_IRQ_MANY_POP_PROGRESS, "blocking") == 0,
          "the interrupt-masked ring gives blocking");
    CHECK(strstr(QUEUE_C_IRQ_MANY_ALGORITHM, "interrupt-masked") != NULL, "the algorithm says why: %s",
          QUEUE_C_IRQ_MANY_ALGORITHM);
}

/* One intrusive queue of either lowering, through the generated functions: the
 * nodes are the caller's array of the generated element, the last one the stub. */
#define EXERCISE_INTRUSIVE(PREFIX, TYPE)                                                                               \
    static void exercise_##PREFIX(void) {                                                                              \
        static TYPE q;                                                                                                 \
        static queue_conformance_node_t nodes[5];                                                                      \
        uint32_t index = 0;                                                                                            \
        unsigned i;                                                                                                    \
        for (i = 0; i < 5u; i++) {                                                                                     \
            memset(&nodes[i], 0, sizeof nodes[i]);                                                                     \
            nodes[i].sensor_id = (uint8_t)(i + 1u);                                                                    \
            nodes[i].value = (uint16_t)(i * 3u);                                                                       \
        }                                                                                                              \
        CHECK(!PREFIX##_init(&q, nodes, 5u, 5u), #PREFIX ": a stub outside the array is refused");                     \
        CHECK(PREFIX##_init(&q, nodes, 5u, 4u), #PREFIX ": the shape the document states is refused");                 \
        CHECK(!PREFIX##_try_pop(&q, &index), #PREFIX ": a new queue is empty");                                        \
        PREFIX##_push(&q, 2u);                                                                                         \
        PREFIX##_push(&q, 0u);                                                                                         \
        PREFIX##_push(&q, 3u);                                                                                         \
        CHECK(PREFIX##_try_pop(&q, &index) && index == 2u, #PREFIX ": first in, first out: 2");                        \
        CHECK(PREFIX##_try_pop(&q, &index) && index == 0u, #PREFIX ": first in, first out: 0");                        \
        PREFIX##_push(&q, 2u);                                                                                         \
        CHECK(PREFIX##_try_pop(&q, &index) && index == 3u, #PREFIX ": first in, first out: 3");                        \
        CHECK(PREFIX##_try_pop(&q, &index) && index == 2u, #PREFIX ": the node comes back at once and can be reused"); \
        CHECK(!PREFIX##_try_pop(&q, &index), #PREFIX ": drained");                                                     \
        for (i = 0; i < 5u; i++) {                                                                                     \
            CHECK(nodes[i].sensor_id == (uint8_t)(i + 1u) && nodes[i].value == (uint16_t)(i * 3u),                     \
                  #PREFIX ": the queue wrote outside the link of node %u", i);                                         \
        }                                                                                                              \
    }

EXERCISE_INTRUSIVE(queue_c_intrusive, queue_c_intrusive_t)
EXERCISE_INTRUSIVE(queue_c_intrusive_irq, queue_c_intrusive_irq_t)

static void the_intrusive_headers_state_what_the_document_required_and_what_it_gives(void) {
    /* Many producers, one consumer over atomics: push is wait-free, and the
     * document could declare no more than blocking because pop is. */
    CHECK(QUEUE_C_INTRUSIVE_MANY_CONSUMERS == 0, "one consumer needs no flag");
    CHECK(strcmp(QUEUE_C_INTRUSIVE_DECLARED_PROGRESS, "blocking") == 0 &&
              strcmp(QUEUE_C_INTRUSIVE_PUSH_PROGRESS, "wait-free") == 0 &&
              strcmp(QUEUE_C_INTRUSIVE_POP_PROGRESS, "blocking") == 0,
          "Vyukov's list: wait-free push, blocking pop");
    CHECK(strstr(QUEUE_C_INTRUSIVE_ALGORITHM, "Vyukov") != NULL, "the algorithm is named: %s",
          QUEUE_C_INTRUSIVE_ALGORITHM);

    /* The same document shape on the target with no atomics runs under the
     * critical section, and everything is blocking there. */
    CHECK(QUEUE_C_INTRUSIVE_IRQ_MANY_CONSUMERS == 1, "many consumers are said");
    CHECK(strcmp(QUEUE_C_INTRUSIVE_IRQ_PUSH_PROGRESS, "blocking") == 0 &&
              strcmp(QUEUE_C_INTRUSIVE_IRQ_POP_PROGRESS, "blocking") == 0,
          "the critical-section list gives blocking");
    CHECK(strstr(QUEUE_C_INTRUSIVE_IRQ_ALGORITHM, "interrupt-masked") != NULL, "the algorithm says why: %s",
          QUEUE_C_INTRUSIVE_IRQ_ALGORITHM);
}

/* ---- The segmented queues ---- */

/* A bump arena over one block: the allocator a segmented queue is injected with on
 * a target that has no heap to speak of. A grant is a compare-and-swap on the
 * offset, so an allocation is lock-free, which is what its `progress` says and what
 * the documents declare for it. Nothing is given back one block at a time: the
 * arena is released whole, which is how such a target frees what a queue held. */
typedef struct arena {
    unsigned char *base;
    size_t len;
    size_t next;
} arena_t;

static void *arena_allocate(void *context, size_t size, size_t align) {
    arena_t *arena = (arena_t *)context;
    size_t used = sce_atomic_load_acquire_usize(&arena->next);
    for (;;) {
        const size_t address = (size_t)(uintptr_t)arena->base + used;
        const size_t padding = (align - address % align) % align;
        const size_t end = used + padding + size;
        size_t seen;
        if (end > arena->len) {
            return NULL;
        }
        seen = sce_atomic_cas_strong_acq_rel_usize(&arena->next, used, end);
        if (seen == used) {
            return arena->base + used + padding;
        }
        used = seen;
    }
}

static void arena_deallocate(void *context, void *block, size_t size, size_t align) {
    (void)context;
    (void)block;
    (void)size;
    (void)align;
}

static sce_queue_allocator_t arena_allocator(arena_t *arena, sce_queue_progress_t progress) {
    sce_queue_allocator_t allocator;
    allocator.context = arena;
    allocator.allocate = arena_allocate;
    allocator.deallocate = arena_deallocate;
    allocator.progress = progress;
    return allocator;
}

/* 64 bytes: the alignment of a segment's cache-line-padded parts needs no more. */
static _Alignas(64) unsigned char g_arena_block[64 * 1024];

static arena_t fresh_arena(size_t len) {
    arena_t arena;
    arena.base = g_arena_block;
    arena.len = len;
    arena.next = 0u;
    return arena;
}

static void the_segmented_headers_state_what_the_document_required_and_what_it_gives(void) {
    CHECK(QUEUE_C_SEGMENTED_SEGMENT == 3u, "the segment is the document's: three");
    CHECK(strcmp(QUEUE_C_SEGMENTED_DECLARED_PROGRESS, "lock-free") == 0 &&
              strcmp(QUEUE_C_SEGMENTED_PUSH_PROGRESS, "lock-free") == 0 &&
              strcmp(QUEUE_C_SEGMENTED_POP_PROGRESS, "wait-free") == 0,
          "linked Lamport rings: a push no stronger than its allocator, a wait-free pop");
    CHECK(strstr(QUEUE_C_SEGMENTED_ALGORITHM, "Lamport") != NULL, "one and one select linked Lamport rings: %s",
          QUEUE_C_SEGMENTED_ALGORITHM);
    CHECK(QUEUE_C_SEGMENTED_ALLOCATOR_PROGRESS == SCE_QUEUE_PROGRESS_LOCK_FREE &&
              strcmp(QUEUE_C_SEGMENTED_ALLOCATOR_PROGRESS_WORD, "lock-free") == 0,
          "the allocator progress is the document's");

    CHECK(QUEUE_C_SEGMENTED_MANY_SEGMENT == 3u && QUEUE_C_SEGMENTED_MANY_PARTICIPANTS == 2u,
          "segment and participants");
    CHECK(QUEUE_C_SEGMENTED_MANY_RING_SLOTS == 4u, "the next power of two at or above max(segment, participants)");
    CHECK(QUEUE_C_SEGMENTED_MANY_HAZARD_SLOTS == 4u, "a slot for every handle on either side");
    CHECK(strcmp(QUEUE_C_SEGMENTED_MANY_PUSH_PROGRESS, "lock-free") == 0 &&
              strcmp(QUEUE_C_SEGMENTED_MANY_POP_PROGRESS, "lock-free") == 0,
          "LSCQ is lock-free on both sides");
    CHECK(strstr(QUEUE_C_SEGMENTED_MANY_ALGORITHM, "LSCQ") != NULL, "many and many select LSCQ: %s",
          QUEUE_C_SEGMENTED_MANY_ALGORITHM);
    CHECK(QUEUE_C_SEGMENTED_MANY_WRAP_BOUND_OPS == SCE_QUEUE_WRAP_BOUND_OPS_64, "2^62 on 64-bit entries");
}

static void the_linked_lamport_queue_hands_events_over_in_order_across_segments(void) {
    static queue_c_segmented_t q;
    arena_t arena = fresh_arena(sizeof g_arena_block);
    sce_queue_allocator_t allocator = arena_allocator(&arena, SCE_QUEUE_PROGRESS_LOCK_FREE);
    queue_conformance_event_t none;
    unsigned i;
    CHECK(queue_c_segmented_init(&q, &allocator), "the arena gives the first segment");
    CHECK(queue_c_segmented_producer_acquire(&q) && queue_c_segmented_consumer_acquire(&q), "one place a side");
    CHECK(!queue_c_segmented_producer_acquire(&q), "there is one producer, and it is taken");
    CHECK(!queue_c_segmented_try_pop(&q, &none), "a new queue is empty");
    /* Three segments' worth and a part: order holds across the links. */
    for (i = 0; i < 8u; i++) {
        const queue_conformance_event_t e = make_event(1u, i);
        CHECK(queue_c_segmented_try_push(&q, &e) == SCE_QUEUE_PUSH_OK, "a push fits the arena");
    }
    for (i = 0; i < 8u; i++) {
        queue_conformance_event_t got;
        CHECK(queue_c_segmented_try_pop(&q, &got) && got.sensor_id == 1u && got.value == i,
              "pop %u: first in, first out", i);
    }
    CHECK(!queue_c_segmented_try_pop(&q, &none), "drained");
    queue_c_segmented_producer_release(&q);
    queue_c_segmented_consumer_release(&q);
    queue_c_segmented_destroy(&q);
}

static void the_lscq_queue_hands_events_over_in_order_across_segments(void) {
    static queue_c_segmented_many_domain_t domain;
    static queue_c_segmented_many_t q;
    arena_t arena = fresh_arena(sizeof g_arena_block);
    sce_queue_allocator_t allocator = arena_allocator(&arena, SCE_QUEUE_PROGRESS_LOCK_FREE);
    uint32_t producer;
    uint32_t consumer;
    uint32_t producer2;
    uint32_t consumer2;
    uint32_t extra;
    queue_conformance_event_t none;
    queue_conformance_event_t got;
    unsigned i;
    CHECK(queue_c_segmented_many_domain_init(&domain), "the domain");
    CHECK(queue_c_segmented_many_init(&q, &domain, &allocator), "the arena gives the first segment");
    CHECK(queue_c_segmented_many_producer_acquire(&q, &producer) &&
              queue_c_segmented_many_consumer_acquire(&q, &consumer),
          "a place a side");
    CHECK(!queue_c_segmented_many_try_pop(&q, consumer, &none), "a new queue is empty");
    for (i = 0; i < 10u; i++) {
        const queue_conformance_event_t e = make_event(2u, i);
        CHECK(queue_c_segmented_many_try_push(&q, producer, &e) == SCE_QUEUE_PUSH_OK, "a push fits the arena");
    }
    for (i = 0; i < 10u; i++) {
        CHECK(queue_c_segmented_many_try_pop(&q, consumer, &got) && got.sensor_id == 2u && got.value == i,
              "pop %u: first in, first out", i);
    }
    CHECK(!queue_c_segmented_many_try_pop(&q, consumer, &none), "drained");
    /* A second producer and a second consumer take the other slots of the domain,
     * which has one for every handle either side can hold. */
    CHECK(queue_c_segmented_many_producer_acquire(&q, &producer2) &&
              queue_c_segmented_many_consumer_acquire(&q, &consumer2),
          "the second handle of each side");
    CHECK(!queue_c_segmented_many_producer_acquire(&q, &extra) && !queue_c_segmented_many_consumer_acquire(&q, &extra),
          "the domain's four slots are taken");
    {
        const queue_conformance_event_t e = make_event(3u, 9u);
        CHECK(queue_c_segmented_many_try_push(&q, producer2, &e) == SCE_QUEUE_PUSH_OK, "the second producer pushes");
    }
    CHECK(queue_c_segmented_many_try_pop(&q, consumer2, &got) && got.sensor_id == 3u && got.value == 9u,
          "and the second consumer pops it");
    queue_c_segmented_many_consumer_release(&q, consumer2);
    queue_c_segmented_many_producer_release(&q, producer2);
    queue_c_segmented_many_consumer_release(&q, consumer);
    queue_c_segmented_many_producer_release(&q, producer);
    queue_c_segmented_many_destroy(&q);
}

static void a_spent_arena_refuses_the_segment_and_the_event_stays_with_its_owner(void) {
    static queue_c_segmented_t q;
    arena_t probe = fresh_arena(sizeof g_arena_block);
    sce_queue_allocator_t probing = arena_allocator(&probe, SCE_QUEUE_PROGRESS_LOCK_FREE);
    arena_t arena;
    sce_queue_allocator_t allocator;
    queue_conformance_event_t refused = make_event(9u, 99u);
    unsigned i;
    /* An arena that holds the first segment and not a second: the push that needs
     * one reports OUT_OF_MEMORY and leaves its element with the caller. */
    CHECK(queue_c_segmented_init(&q, &probing), "a probe queue");
    queue_c_segmented_destroy(&q);
    arena = fresh_arena(probe.next);
    allocator = arena_allocator(&arena, SCE_QUEUE_PROGRESS_LOCK_FREE);
    CHECK(queue_c_segmented_init(&q, &allocator), "the arena holds exactly the first segment");
    CHECK(queue_c_segmented_producer_acquire(&q) && queue_c_segmented_consumer_acquire(&q), "one place a side");
    for (i = 0; i < QUEUE_C_SEGMENTED_SEGMENT; i++) {
        const queue_conformance_event_t e = make_event(1u, i);
        CHECK(queue_c_segmented_try_push(&q, &e) == SCE_QUEUE_PUSH_OK, "the first segment holds a segment");
    }
    CHECK(queue_c_segmented_try_push(&q, &refused) == SCE_QUEUE_PUSH_OUT_OF_MEMORY, "the next needs a second segment");
    CHECK(refused.sensor_id == 9u && refused.value == 99u, "the refused event is as it was");
    for (i = 0; i < QUEUE_C_SEGMENTED_SEGMENT; i++) {
        queue_conformance_event_t got;
        CHECK(queue_c_segmented_try_pop(&q, &got) && got.sensor_id == 1u && got.value == i,
              "what fit comes out in order");
    }
    queue_c_segmented_producer_release(&q);
    queue_c_segmented_consumer_release(&q);
    queue_c_segmented_destroy(&q);
}

static void an_allocator_that_gives_less_than_the_document_declared_is_refused(void) {
    static queue_c_segmented_t linked;
    static queue_c_segmented_many_domain_t domain;
    static queue_c_segmented_many_t many;
    arena_t arena = fresh_arena(sizeof g_arena_block);
    sce_queue_allocator_t blocking = arena_allocator(&arena, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_queue_allocator_t wait_free = arena_allocator(&arena, SCE_QUEUE_PROGRESS_WAIT_FREE);
    CHECK(!queue_c_segmented_init(&linked, &blocking), "linked: a blocking allocator under a lock-free declaration");
    CHECK(queue_c_segmented_many_domain_init(&domain), "the domain");
    CHECK(!queue_c_segmented_many_init(&many, &domain, &blocking),
          "lscq: a blocking allocator under a lock-free declaration");
    CHECK(arena.next == 0u, "a refused construction takes no block");
    CHECK(queue_c_segmented_many_init(&many, &domain, &wait_free), "a wait-free allocator is more than enough");
    queue_c_segmented_many_destroy(&many);
}

static void a_many_side_has_the_documents_participants_as_its_places(void) {
    static queue_c_scq64_t scq;
    static queue_c_irq_many_t irq;
    unsigned taken;
    CHECK(queue_c_scq64_init(&scq), "scq64 init");
    for (taken = 0; taken < QUEUE_C_SCQ64_RING_SLOTS; taken++) {
        CHECK(queue_c_scq64_producer_acquire(&scq), "scq64 producer %u", taken);
    }
    CHECK(!queue_c_scq64_producer_acquire(&scq), "a ring cannot be shared by more enqueuers than it has slots");
    CHECK(queue_c_irq_many_init(&irq), "irq init");
    for (taken = 0; taken < QUEUE_C_IRQ_MANY_PARTICIPANTS; taken++) {
        CHECK(queue_c_irq_many_producer_acquire(&irq), "irq producer %u", taken);
    }
    CHECK(!queue_c_irq_many_producer_acquire(&irq),
          "the interrupt-masked ring has the documents participants as places");
}

int main(void) {
    exercise_queue_c_spsc();
    exercise_queue_c_scq64();
    exercise_queue_c_scq32();
    exercise_queue_c_irq_one();
    exercise_queue_c_irq_many();
    exercise_queue_c_intrusive();
    exercise_queue_c_intrusive_irq();
    the_intrusive_headers_state_what_the_document_required_and_what_it_gives();
    the_segmented_headers_state_what_the_document_required_and_what_it_gives();
    the_linked_lamport_queue_hands_events_over_in_order_across_segments();
    the_lscq_queue_hands_events_over_in_order_across_segments();
    a_spent_arena_refuses_the_segment_and_the_event_stays_with_its_owner();
    an_allocator_that_gives_less_than_the_document_declared_is_refused();
    each_header_states_what_the_document_required_and_what_it_gives();
    a_many_side_has_the_documents_participants_as_its_places();
    (void)printf("generated queues (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
