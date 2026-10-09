/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the runtime half of `sce:kind="queue"` (SCE Protocol-Synthesis
 * RFC §synth-5-P) for C11: queues that hand elements from one thread, core or
 * interrupt handler to another.
 *
 * The document states a contract and names no algorithm; each algorithm the
 * kind's selection table names lives here once. The contract every one of them
 * keeps: a linearizable FIFO in which the push of an element happens-before the
 * pop that returns it, never holding more than the capacity declared and
 * holding exactly that when nothing is running. The SCQ row may refuse a push
 * while another participant's operation holds a slot (at most one per
 * participant); the Lamport row never does.
 *
 *   - sce_queue_spsc_*      the `bounded` row for one producer and one consumer:
 *                           a Lamport ring, wait-free on both sides.
 *   - sce_queue_scq64_*     the `bounded` row for any other cardinality, on a
 *                           target whose atomics reach 64 bits: Nikolaev's SCQ
 *                           data queue (DISC 2019), lock-free on both sides.
 *   - sce_queue_scq32_*     the same on a target whose atomics reach only 32
 *                           bits. Its entry word leaves the cycle fewer bits, so
 *                           its WRAP_BOUND_OPS is 2^30 and not 2^62 (RFC §synth-5-P,
 *                           Counter width), and the deploy states the delay it
 *                           relies on.
 *   - sce_queue_irq_*       any `bounded` row on a target with no read-modify-write
 *                           atomics at all (`platform.atomic_rmw_width` 0): one
 *                           ring under an interrupt-masked critical section,
 *                           `blocking`. It lives in sce/forge/queue_irq.h, which
 *                           needs a platform-defined `irq_state_t`.
 *
 * The atomics are the RFC §synth-5-I `sce_atomic_*` externs (sce/forge/atomics.h),
 * which the platform implements; this header contains no compiler builtin and no
 * <stdatomic.h>, so the same text runs on a hosted target and on a bare-metal one.
 *
 * What differs from the languages with destructors and generics:
 *
 *   - The storage is the caller's. A queue is a control block plus an array of
 *     `capacity` slots of `element_size` bytes, which the generated type for the
 *     document lays out statically, so nothing here allocates and nothing needs
 *     the heap on the no-alloc profile. Elements are copied in and out by
 *     memcpy: an element of this kind is a plain value (the generated codec
 *     struct), with no destructor to run and no reference to release.
 *   - A queue is built by *_init and handed to the other participants after it
 *     returns. Nothing orders *_init against a concurrent user.
 *   - A place of a side (a producer or a consumer) is taken with *_acquire and
 *     given back with *_release. The Lamport row has one of each.
 */

#ifndef SCE_FORGE_QUEUE_H
#define SCE_FORGE_QUEUE_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <sce/forge/atomics.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Whether a push took its element. OUT_OF_MEMORY is the `segmented` storage
 * mode's, reported when its injected allocator refuses a segment; a `bounded`
 * queue only ever reports FULL. */
typedef enum sce_queue_push_status {
    SCE_QUEUE_PUSH_OK = 0,
    SCE_QUEUE_PUSH_FULL = 1,
    SCE_QUEUE_PUSH_OUT_OF_MEMORY = 2
} sce_queue_push_status_t;

/* How many operations a participant may be delayed between reading an entry and
 * acting on it before it could be misled: 2^(w-2) for an entry word of w bits,
 * independent of the capacity (RFC §synth-5-P, Counter width). */
#define SCE_QUEUE_WRAP_BOUND_OPS_64 (((uint64_t)1) << 62)
#define SCE_QUEUE_WRAP_BOUND_OPS_32 (((uint64_t)1) << 30)

/* Takes one of `limit` places of a side, if one is free. A compare-and-swap loop
 * and not an add, so a refused request leaves the count as it was and no
 * concurrent request can be refused on account of it. Returns nonzero when it
 * took one. */
static inline int sce_queue_take_place(uint32_t *alive, uint32_t limit) {
    for (;;) {
        const uint32_t seen = sce_atomic_load_acquire_u32(alive);
        if (seen >= limit) {
            return 0;
        }
        if (sce_atomic_cas_strong_acq_rel_u32(alive, seen, seen + 1u) == seen) {
            return 1;
        }
    }
}

/* ------------------------------------------------------------------------ */
/* The Lamport ring: one producer, one consumer, wait-free.                   */
/* ------------------------------------------------------------------------ */

/* A bounded queue of exactly `capacity` elements for one producer and one
 * consumer.
 *
 * Lap indices, not counters. Each side's index runs over 0..2N and the slot it
 * names is the index modulo N. Two laps are what tell a full ring from an empty
 * one without spending a slot: the indices are equal exactly when the ring is
 * empty and N apart exactly when it is full, so the ring holds exactly the
 * capacity declared. An index never exceeds 2N, so nothing wraps however long
 * the queue runs; 2N must fit in 32 bits, which sce_queue_spsc_init checks.
 *
 * Ordering. The producer publishes a filled slot by storing its index (release)
 * and the consumer reads that index (acquire) before it reads the slot, so the
 * write of an element happens-before its pop; the consumer returns a slot the
 * same way in the other direction. Each side keeps the other's index as last
 * read, and reads the shared one again only when the cached one says the ring
 * is full (producer) or empty (consumer). */
typedef struct sce_queue_spsc {
    /* The lap index of the next slot the consumer pops. Written only by the
     * consumer. */
    uint32_t head;
    /* The lap index of the next slot the producer fills. Written only by the
     * producer. */
    uint32_t tail;
    uint32_t producer_claimed;
    uint32_t consumer_claimed;
    uint32_t capacity;
    /* The size of the lap-index space, twice the capacity. */
    uint32_t laps;
    /* The producer's own index, and the consumer's index as it last read it.
     * Private to the producer; set by sce_queue_spsc_producer_acquire. */
    uint32_t producer_tail;
    uint32_t producer_cached_head;
    /* The consumer's own index, and the producer's index as it last read it.
     * Private to the consumer. */
    uint32_t consumer_head;
    uint32_t consumer_cached_tail;
} sce_queue_spsc_t;

/* Builds an empty queue of exactly `capacity` elements. Returns zero, and builds
 * nothing, for a capacity below one or one whose lap-index space (2N) does not
 * fit in 32 bits. */
static inline int sce_queue_spsc_init(sce_queue_spsc_t *q, uint32_t capacity) {
    if (capacity < 1u || capacity > 0x7FFFFFFFu) {
        return 0;
    }
    q->head = 0;
    q->tail = 0;
    q->producer_claimed = 0;
    q->consumer_claimed = 0;
    q->capacity = capacity;
    q->laps = 2u * capacity;
    q->producer_tail = 0;
    q->producer_cached_head = 0;
    q->consumer_head = 0;
    q->consumer_cached_tail = 0;
    return 1;
}

static inline uint32_t sce_queue_spsc_next(const sce_queue_spsc_t *q, uint32_t index) {
    return index + 1u == q->laps ? 0u : index + 1u;
}

static inline uint32_t sce_queue_spsc_slot(const sce_queue_spsc_t *q, uint32_t index) {
    return index >= q->capacity ? index - q->capacity : index;
}

/* How many elements lie between a consumer index and a producer index. */
static inline uint32_t sce_queue_spsc_occupancy(const sce_queue_spsc_t *q, uint32_t head, uint32_t tail) {
    return tail >= head ? tail - head : tail + q->laps - head;
}

/* Takes the producing side, or returns zero while another is held: a ring shared
 * by two producers is not this algorithm. */
static inline int sce_queue_spsc_producer_acquire(sce_queue_spsc_t *q) {
    if (sce_atomic_cas_strong_acq_rel_u32(&q->producer_claimed, 0u, 1u) != 0u) {
        return 0;
    }
    q->producer_tail = sce_atomic_load_acquire_u32(&q->tail);
    q->producer_cached_head = sce_atomic_load_acquire_u32(&q->head);
    return 1;
}

/* Gives the producing side back. */
static inline void sce_queue_spsc_producer_release(sce_queue_spsc_t *q) {
    sce_atomic_store_release_u32(&q->producer_claimed, 0u);
}

/* Takes the consuming side, or returns zero while another is held. */
static inline int sce_queue_spsc_consumer_acquire(sce_queue_spsc_t *q) {
    if (sce_atomic_cas_strong_acq_rel_u32(&q->consumer_claimed, 0u, 1u) != 0u) {
        return 0;
    }
    q->consumer_head = sce_atomic_load_acquire_u32(&q->head);
    q->consumer_cached_tail = sce_atomic_load_acquire_u32(&q->tail);
    return 1;
}

/* Gives the consuming side back. */
static inline void sce_queue_spsc_consumer_release(sce_queue_spsc_t *q) {
    sce_atomic_store_release_u32(&q->consumer_claimed, 0u);
}

/* Takes `*value` (element_size bytes) into slot storage `slots` (capacity slots
 * of element_size bytes), or reports full when the queue holds its capacity.
 * Wait-free: a bounded number of steps whatever the consumer is doing. */
static inline sce_queue_push_status_t sce_queue_spsc_try_push(sce_queue_spsc_t *q, unsigned char *slots,
                                                              size_t element_size, const void *value) {
    if (sce_queue_spsc_occupancy(q, q->producer_cached_head, q->producer_tail) == q->capacity) {
        /* Read the consumer's index again: its read of the slot about to be
         * reused happens-before the write below. */
        q->producer_cached_head = sce_atomic_load_acquire_u32(&q->head);
        if (sce_queue_spsc_occupancy(q, q->producer_cached_head, q->producer_tail) == q->capacity) {
            return SCE_QUEUE_PUSH_FULL;
        }
    }
    /* The slot at tail is outside [head, tail), so the consumer does not read it
     * until the store below publishes it. */
    memcpy(slots + (size_t)sce_queue_spsc_slot(q, q->producer_tail) * element_size, value, element_size);
    q->producer_tail = sce_queue_spsc_next(q, q->producer_tail);
    /* The write of the slot happens-before the pop of any consumer that reads
     * this index. */
    sce_atomic_store_release_u32(&q->tail, q->producer_tail);
    return SCE_QUEUE_PUSH_OK;
}

/* Takes the oldest element into `*out`, or returns zero when the queue is empty.
 * Wait-free: a bounded number of steps whatever the producer is doing. */
static inline int sce_queue_spsc_try_pop(sce_queue_spsc_t *q, unsigned char *slots, size_t element_size, void *out) {
    if (q->consumer_head == q->consumer_cached_tail) {
        /* Read the producer's index again: its write of the slot happens-before
         * the read below. */
        q->consumer_cached_tail = sce_atomic_load_acquire_u32(&q->tail);
        if (q->consumer_head == q->consumer_cached_tail) {
            return 0;
        }
    }
    /* The slot at head is inside [head, tail), so it holds a pushed element, and
     * the producer does not reuse it until the store below returns it. */
    memcpy(out, slots + (size_t)sce_queue_spsc_slot(q, q->consumer_head) * element_size, element_size);
    q->consumer_head = sce_queue_spsc_next(q, q->consumer_head);
    /* The read of the slot happens-before the producer's reuse of it, by any
     * producer that reads this index. */
    sce_atomic_store_release_u32(&q->head, q->consumer_head);
    return 1;
}

/* ------------------------------------------------------------------------ */
/* SCQ: any other cardinality, lock-free. One definition, two entry widths.   */
/* ------------------------------------------------------------------------ */

/* The 64-bit-entry queue: a ring of up to 2^30 slots leaves the cycle 32 bits. */
#define SCE_QSCQ_WORD uint64_t
#define SCE_QSCQ_SWORD int64_t
#define SCE_QSCQ_BITS 64
#define SCE_QSCQ_MAX_RING_SLOTS (1u << 30)
#define SCE_QSCQ_PREFIX sce_queue_scq64_
#define SCE_QSCQ_LOAD_ACQ sce_atomic_load_acquire_u64
#define SCE_QSCQ_LOAD_SEQ sce_atomic_load_seq_cst_u64
#define SCE_QSCQ_STORE_SEQ sce_atomic_store_seq_cst_u64
#define SCE_QSCQ_CAS sce_atomic_cas_strong_acq_rel_u64
#define SCE_QSCQ_FETCH_ADD sce_atomic_fetch_add_acq_rel_u64
#define SCE_QSCQ_FETCH_SUB sce_atomic_fetch_sub_acq_rel_u64
#define SCE_QSCQ_FETCH_OR sce_atomic_fetch_or_acq_rel_u64
#include <sce/forge/queue_scq_width.inc>

/* The 32-bit-entry queue: a ring of up to 2^15 slots leaves the cycle 15 bits. */
#define SCE_QSCQ_WORD uint32_t
#define SCE_QSCQ_SWORD int32_t
#define SCE_QSCQ_BITS 32
#define SCE_QSCQ_MAX_RING_SLOTS (1u << 15)
#define SCE_QSCQ_PREFIX sce_queue_scq32_
#define SCE_QSCQ_LOAD_ACQ sce_atomic_load_acquire_u32
#define SCE_QSCQ_LOAD_SEQ sce_atomic_load_seq_cst_u32
#define SCE_QSCQ_STORE_SEQ sce_atomic_store_seq_cst_u32
#define SCE_QSCQ_CAS sce_atomic_cas_strong_acq_rel_u32
#define SCE_QSCQ_FETCH_ADD sce_atomic_fetch_add_acq_rel_u32
#define SCE_QSCQ_FETCH_SUB sce_atomic_fetch_sub_acq_rel_u32
#define SCE_QSCQ_FETCH_OR sce_atomic_fetch_or_acq_rel_u32
#include <sce/forge/queue_scq_width.inc>

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_QUEUE_H */
