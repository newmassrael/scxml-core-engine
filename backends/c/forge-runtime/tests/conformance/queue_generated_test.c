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

#include "queue_c_irq_many.h"
#include "queue_c_irq_one.h"
#include "queue_c_scq32.h"
#include "queue_c_scq64.h"
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
    each_header_states_what_the_document_required_and_what_it_gives();
    a_many_side_has_the_documents_participants_as_its_places();
    (void)printf("generated queues (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
