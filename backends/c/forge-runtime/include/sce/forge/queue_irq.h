/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the queue for a target with no read-modify-write atomics (SCE
 * Protocol-Synthesis RFC §synth-5-P, Backends; `platform.atomic_rmw_width` 0,
 * ARMv6-M for instance).
 *
 * Every operation runs inside an `sce_irq_save` / `sce_irq_restore` critical
 * section (RFC §synth-5-I, interrupt control), so the queue gives `blocking` for
 * every storage mode and cardinality: an interrupt that cannot run while the
 * section is open is the participant that waits. Masking interrupts excludes
 * nothing on another core, so such a target is supported only with
 * `platform.core_count` 1, and the generator refuses any other
 * (`queue/no-atomics-across-cores`). On the single core an interrupt handler can
 * never preempt inside the section, which is why the ISR checks are met here
 * although the row is `blocking`.
 *
 * `irq_state_t` is the type of the saved interrupt state. The RFC leaves it to
 * the target (`sce_irq_save() -> irq_state_t`), and this header has to be
 * consumable on its own, so it carries a default: `uint32_t`, which holds the
 * interrupt mask of a Cortex-M (PRIMASK) and of most 32-bit cores. A target
 * whose saved state is another type defines its own `irq_state_t` and
 * SCE_IRQ_STATE_T_DEFINED before this header is included, and the default is
 * then not declared.
 */

#ifndef SCE_FORGE_QUEUE_IRQ_H
#define SCE_FORGE_QUEUE_IRQ_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <sce/forge/queue.h>

#ifndef SCE_IRQ_STATE_T_DEFINED
typedef uint32_t irq_state_t;
#endif

#ifdef __cplusplus
extern "C" {
#endif

extern irq_state_t sce_irq_save(void);
extern void sce_irq_restore(irq_state_t p0);

/* A bounded queue of exactly `capacity` elements: one ring, one critical
 * section per operation. `producer_places` and `consumer_places` are how many
 * places of each side may be held at once. */
typedef struct sce_queue_irq {
    uint32_t head;
    uint32_t count;
    uint32_t capacity;
    uint32_t producers;
    uint32_t consumers;
    uint32_t producer_places;
    uint32_t consumer_places;
} sce_queue_irq_t;

/* Builds an empty queue. Returns zero, and builds nothing, for a capacity below
 * one or a side with no place. */
static inline int sce_queue_irq_init(sce_queue_irq_t *q, uint32_t capacity, uint32_t producer_places,
                                     uint32_t consumer_places) {
    if (capacity < 1u || producer_places < 1u || consumer_places < 1u) {
        return 0;
    }
    q->head = 0;
    q->count = 0;
    q->capacity = capacity;
    q->producers = 0;
    q->consumers = 0;
    q->producer_places = producer_places;
    q->consumer_places = consumer_places;
    return 1;
}

static inline int sce_queue_irq_producer_acquire(sce_queue_irq_t *q) {
    int taken = 0;
    const irq_state_t state = sce_irq_save();
    if (q->producers < q->producer_places) {
        q->producers++;
        taken = 1;
    }
    sce_irq_restore(state);
    return taken;
}

static inline void sce_queue_irq_producer_release(sce_queue_irq_t *q) {
    const irq_state_t state = sce_irq_save();
    q->producers--;
    sce_irq_restore(state);
}

static inline int sce_queue_irq_consumer_acquire(sce_queue_irq_t *q) {
    int taken = 0;
    const irq_state_t state = sce_irq_save();
    if (q->consumers < q->consumer_places) {
        q->consumers++;
        taken = 1;
    }
    sce_irq_restore(state);
    return taken;
}

static inline void sce_queue_irq_consumer_release(sce_queue_irq_t *q) {
    const irq_state_t state = sce_irq_save();
    q->consumers--;
    sce_irq_restore(state);
}

/* Takes `*value` (element_size bytes) into slot storage `slots`, or reports full
 * when the queue holds its capacity. Blocking: the critical section. */
static inline sce_queue_push_status_t sce_queue_irq_try_push(sce_queue_irq_t *q, unsigned char *slots,
                                                             size_t element_size, const void *value) {
    sce_queue_push_status_t status = SCE_QUEUE_PUSH_FULL;
    const irq_state_t state = sce_irq_save();
    if (q->count < q->capacity) {
        const uint32_t at = (q->head + q->count) % q->capacity;
        memcpy(slots + (size_t)at * element_size, value, element_size);
        q->count++;
        status = SCE_QUEUE_PUSH_OK;
    }
    sce_irq_restore(state);
    return status;
}

/* Takes the oldest element into `*out`, or returns zero when the queue is
 * empty. Blocking: the critical section. */
static inline int sce_queue_irq_try_pop(sce_queue_irq_t *q, unsigned char *slots, size_t element_size, void *out) {
    int popped = 0;
    const irq_state_t state = sce_irq_save();
    if (q->count > 0u) {
        memcpy(out, slots + (size_t)q->head * element_size, element_size);
        q->head = (q->head + 1u) % q->capacity;
        q->count--;
        popped = 1;
    }
    sce_irq_restore(state);
    return popped;
}

/* The intrusive list (sce/forge/queue.h, sce_queue_intrusive_*) for the same
 * target: a plain singly linked list whose every operation runs in the critical
 * section. The links are the same `uint32_t` node indices, in the same place, so
 * the generated type does not change with the target. The critical section
 * excludes every other participant, so nothing can be caught between two steps:
 * a pop never answers empty while a node is queued, the stub is not needed and
 * stays out of the list, and the consumers need no flag. */
typedef struct sce_queue_intrusive_irq {
    uint32_t head;
    uint32_t tail;
    uint32_t len;
    size_t stride;
    size_t link_offset;
    unsigned char *nodes;
} sce_queue_intrusive_irq_t;

static inline uint32_t *sce_queue_intrusive_irq_link(const sce_queue_intrusive_irq_t *q, uint32_t index) {
    return (uint32_t *)(q->nodes + (size_t)index * q->stride + q->link_offset);
}

/* Builds an empty list over `len` nodes at `nodes`; takes what
 * sce_queue_intrusive_init takes, and refuses what it refuses. `stub` is checked
 * and otherwise unused, and `many_consumers` is not needed. */
static inline int sce_queue_intrusive_irq_init(sce_queue_intrusive_irq_t *q, void *nodes, uint32_t len, size_t stride,
                                               size_t link_offset, uint32_t stub, int many_consumers) {
    (void)many_consumers;
    if (nodes == NULL || stub >= len || len >= SCE_QUEUE_NIL || link_offset + sizeof(uint32_t) > stride ||
        link_offset % sizeof(uint32_t) != 0u || stride % sizeof(uint32_t) != 0u) {
        return 0;
    }
    q->head = SCE_QUEUE_NIL;
    q->tail = SCE_QUEUE_NIL;
    q->len = len;
    q->stride = stride;
    q->link_offset = link_offset;
    q->nodes = (unsigned char *)nodes;
    return 1;
}

/* Puts node `index` on the list. Blocking: the critical section. */
static inline void sce_queue_intrusive_irq_push(sce_queue_intrusive_irq_t *q, uint32_t index) {
    *sce_queue_intrusive_irq_link(q, index) = SCE_QUEUE_NIL;
    const irq_state_t state = sce_irq_save();
    if (q->tail == SCE_QUEUE_NIL) {
        q->head = index;
    } else {
        *sce_queue_intrusive_irq_link(q, q->tail) = index;
    }
    q->tail = index;
    sce_irq_restore(state);
}

/* Takes the oldest node into `*index`, or returns zero when the list is empty.
 * Blocking: the critical section. */
static inline int sce_queue_intrusive_irq_try_pop(sce_queue_intrusive_irq_t *q, uint32_t *index) {
    int popped = 0;
    const irq_state_t state = sce_irq_save();
    if (q->head != SCE_QUEUE_NIL) {
        *index = q->head;
        q->head = *sce_queue_intrusive_irq_link(q, q->head);
        if (q->head == SCE_QUEUE_NIL) {
            q->tail = SCE_QUEUE_NIL;
        }
        popped = 1;
    }
    sce_irq_restore(state);
    return popped;
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_QUEUE_IRQ_H */
