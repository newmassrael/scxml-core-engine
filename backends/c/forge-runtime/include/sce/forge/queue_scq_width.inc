/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * One SCQ queue (SCE Protocol-Synthesis RFC §synth-5-P) over an entry word of
 * one width. sce/forge/queue.h includes this file twice, once for 64-bit
 * entries and once for 32-bit ones, and nothing else includes it: the
 * algorithm is written once and the width is a parameter, because the entry
 * is `cycle | IsSafe | index` in one atomic word and a word of `w` bits leaves
 * the cycle `w - 1 - log2(2R)` bits (RFC §synth-5-P, Counter width).
 *
 * The including header defines, before the include:
 *
 *   SCE_QSCQ_WORD, SCE_QSCQ_SWORD   the unsigned entry word and its signed twin
 *   SCE_QSCQ_BITS                   its width
 *   SCE_QSCQ_MAX_RING_SLOTS         the largest ring that leaves a cycle enough
 *                                   bits
 *   SCE_QSCQ_PREFIX                 the identifier prefix (`sce_queue_scq64_`)
 *   SCE_QSCQ_LOAD_ACQ, SCE_QSCQ_LOAD_SEQ, SCE_QSCQ_STORE_SEQ, SCE_QSCQ_CAS,
 *   SCE_QSCQ_FETCH_ADD, SCE_QSCQ_FETCH_SUB, SCE_QSCQ_FETCH_OR
 *                                   the atomics of that width (atomics.h)
 *   SCE_QSCQ_CLOSABLE               1 when a ring of this width may be closed
 *                                   (the `segmented` queue's segments), else 0
 *
 * and this file undefines them all at its end.
 *
 * A ring is closed by a bit in the allocated ring's tail, the top bit of the
 * word, which the width must be able to spare: the tickets are the bits below
 * it, and a ring whose counter could reach that bit would close itself. A
 * 64-bit ring never reaches it (SCE_QUEUE_WRAP_BOUND_OPS_64); a 32-bit one
 * would after 2^31 operations, half of what its bounded queue is built for, so
 * the 32-bit ring is not closable and the close functions are not defined for
 * it.
 *
 * It is Nikolaev's SCQ data queue (DISC 2019) in the form of the authors'
 * single-width-CAS reference implementation (lfring_cas1.h, dual 2-clause BSD
 * / MIT), with the Rust runtime's memory orders: acquire loads, acq_rel
 * read-modify-writes, and a sequentially consistent emptiness threshold. See
 * backends/rust/forge-runtime/src/queue/scq.rs and
 * backends/cpp/forge-runtime/include/sce/forge/queue.h for the same code in
 * the two languages whose verification (loom, Miri, ThreadSanitizer, the
 * histories) this one is held to.
 */

#define SCE_QSCQ_PASTE_(a, b) a##b
#define SCE_QSCQ_PASTE(a, b) SCE_QSCQ_PASTE_(a, b)
#define SCE_QSCQ_NAME(name) SCE_QSCQ_PASTE(SCE_QSCQ_PREFIX, name)

/* The tail's top bit, set once a closable ring is closed: an enqueue that takes a
 * ticket from a closed tail gets the bit back and puts nothing on the ring. This
 * is the finalize bit of Nikolaev's LSCQ. A ring that is not closable has no such
 * bit, and every test of it is a test of zero. */
#if SCE_QSCQ_CLOSABLE
#define SCE_QSCQ_FIN ((SCE_QSCQ_WORD)((SCE_QSCQ_WORD)1 << (SCE_QSCQ_BITS - 1)))
#else
#define SCE_QSCQ_FIN ((SCE_QSCQ_WORD)0)
#endif

/* How many entries one 64-byte cache line holds, as a power of two. Consecutive
 * tickets are spread over lines by rotating the ticket's bits by this much; a
 * ring smaller than a line does not rotate. */
#define SCE_QSCQ_LINE_SHIFT 3u

/* How often a dequeue that found an entry still empty looks again before it
 * makes the entry unusable for the enqueue that owns it. Bounded, so the
 * operation stays lock-free. */
#define SCE_QSCQ_SPINS 10000u

/* One SCQ ring of R slots (R a power of two) and 2R entries. An entry is
 * `cycle | IsSafe | index`: the index in the low log2(2R) bits, the IsSafe bit
 * above it, the cycle in the rest. An entry whose index bits are all ones holds
 * nothing. The words are unsigned; nothing in the algorithm orders them but the
 * signed difference of two of them (`before`), and every shift that must not
 * extend a sign is on an unsigned word. */
typedef struct SCE_QSCQ_NAME(ring) {
    /* The next ticket a dequeue takes. */
    SCE_QSCQ_WORD head;
    /* How many failed dequeues remain before the ring may be called empty,
     * held as a signed value in a word; negative means it is empty. */
    SCE_QSCQ_WORD threshold;
    /* The next ticket an enqueue takes. */
    SCE_QSCQ_WORD tail;
    /* The 2R entries, owned by the caller. */
    SCE_QSCQ_WORD *entries;
    /* 2R; also the value of the IsSafe bit. */
    SCE_QSCQ_WORD entry_count;
    /* The bits an index occupies. */
    SCE_QSCQ_WORD index_mask;
    /* The index bits and the IsSafe bit: everything below the cycle. */
    SCE_QSCQ_WORD low_mask;
    /* 3R - 1: that many failed dequeues are enough to show that an
     * empty-looking ring is empty. */
    SCE_QSCQ_WORD threshold_after_enqueue;
    /* log2(2R). */
    unsigned entry_order;
} SCE_QSCQ_NAME(ring_t);

/* Whether `x` is before `y` in the order that wraps: the signed difference.
 * Tickets and cycles are compared this way throughout. */
static inline int SCE_QSCQ_NAME(before)(SCE_QSCQ_WORD x, SCE_QSCQ_WORD y) {
    return (SCE_QSCQ_SWORD)(SCE_QSCQ_WORD)(x - y) < 0;
}

static inline int SCE_QSCQ_NAME(at_or_before)(SCE_QSCQ_WORD x, SCE_QSCQ_WORD y) {
    return (SCE_QSCQ_SWORD)(SCE_QSCQ_WORD)(x - y) <= 0;
}

/* The entry a ticket names: the ticket rotated so that consecutive tickets land
 * on different cache lines. */
static inline SCE_QSCQ_WORD SCE_QSCQ_NAME(map_ticket)(const SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD ticket) {
    if (r->entry_order >= SCE_QSCQ_LINE_SHIFT) {
        return ((ticket & r->index_mask) >> (r->entry_order - SCE_QSCQ_LINE_SHIFT)) |
               ((SCE_QSCQ_WORD)(ticket << SCE_QSCQ_LINE_SHIFT) & r->index_mask);
    }
    return ticket & r->index_mask;
}

/* The ticket `map_ticket` sends to this entry. */
static inline SCE_QSCQ_WORD SCE_QSCQ_NAME(unmap)(const SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD position) {
    if (r->entry_order >= SCE_QSCQ_LINE_SHIFT) {
        return ((position & r->index_mask) >> SCE_QSCQ_LINE_SHIFT) |
               ((SCE_QSCQ_WORD)(position << (r->entry_order - SCE_QSCQ_LINE_SHIFT)) & r->index_mask);
    }
    return position & r->index_mask;
}

/* Builds a ring of `slots` slots over `entries` (2 * slots words) holding the
 * indices 0..filled, oldest first. Not safe against a concurrent user: the ring
 * is published to the other threads after this returns. */
static inline void SCE_QSCQ_NAME(ring_init)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD *entries, SCE_QSCQ_WORD slots,
                                            SCE_QSCQ_WORD filled) {
    SCE_QSCQ_WORD position;
    r->entries = entries;
    r->entry_count = (SCE_QSCQ_WORD)(2u * slots);
    r->index_mask = (SCE_QSCQ_WORD)(r->entry_count - 1u);
    r->low_mask = (SCE_QSCQ_WORD)(2u * r->entry_count - 1u);
    r->threshold_after_enqueue = (SCE_QSCQ_WORD)(3u * slots - 1u);
    r->entry_order = 0;
    while (((SCE_QSCQ_WORD)1 << r->entry_order) < r->entry_count) {
        r->entry_order++;
    }
    for (position = 0; position < r->entry_count; position++) {
        /* The first `filled` tickets' entries hold index ticket in cycle 0,
         * safe; the rest hold nothing, in the cycle before it. */
        SCE_QSCQ_WORD ticket = SCE_QSCQ_NAME(unmap)(r, position);
        entries[position] =
            ticket < filled ? (SCE_QSCQ_WORD)(r->entry_count + ticket) : (SCE_QSCQ_WORD) ~(SCE_QSCQ_WORD)0;
    }
    r->head = 0;
    r->tail = filled;
    r->threshold = filled == 0 ? (SCE_QSCQ_WORD) ~(SCE_QSCQ_WORD)0 : r->threshold_after_enqueue;
}

#if SCE_QSCQ_CLOSABLE
/* Closes the ring: an enqueue that takes its ticket after this one's
 * fetch-and-or puts nothing on it. An enqueue that already holds a ticket may
 * still complete; the dequeues that follow either find its entry or make it
 * unusable, which sends that enqueue to the next ticket and so to a refusal. */
static inline void SCE_QSCQ_NAME(ring_close)(SCE_QSCQ_NAME(ring_t) * r) {
    (void)SCE_QSCQ_FETCH_OR(&r->tail, SCE_QSCQ_FIN);
}
#endif

/* Puts `index` on the ring and returns nonzero, or returns zero, putting nothing,
 * when the ring is closed. An open ring never refuses: at most R indices circulate
 * through it, and an enqueue that finds its entry unusable takes the next
 * ticket. */
static inline int SCE_QSCQ_NAME(enqueue)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD index) {
    const SCE_QSCQ_WORD stored = (SCE_QSCQ_WORD)(index ^ r->index_mask);
    for (;;) {
        SCE_QSCQ_WORD tail;
        /* Looking first keeps a closed ring's tail from counting attempts that
         * cannot succeed; the ticket's own bit is what decides. */
        if ((SCE_QSCQ_LOAD_ACQ(&r->tail) & SCE_QSCQ_FIN) != 0) {
            return 0;
        }
        tail = SCE_QSCQ_FETCH_ADD(&r->tail, 1);
        if ((tail & SCE_QSCQ_FIN) != 0) {
            return 0;
        }
        const SCE_QSCQ_WORD ticket_cycle = (SCE_QSCQ_WORD)((SCE_QSCQ_WORD)(tail << 1) | r->low_mask);
        SCE_QSCQ_WORD *slot = &r->entries[SCE_QSCQ_NAME(map_ticket)(r, tail)];
        SCE_QSCQ_WORD entry = SCE_QSCQ_LOAD_ACQ(slot);
        /* The entry is ours to fill when it is empty, from an earlier cycle, and
         * either safe or not yet passed by the head. */
        for (;;) {
            const SCE_QSCQ_WORD entry_cycle = (SCE_QSCQ_WORD)(entry | r->low_mask);
            const int usable =
                SCE_QSCQ_NAME(before)(entry_cycle, ticket_cycle) &&
                (entry == entry_cycle || (entry == (SCE_QSCQ_WORD)(entry_cycle ^ r->entry_count) &&
                                          SCE_QSCQ_NAME(at_or_before)(SCE_QSCQ_LOAD_ACQ(&r->head), tail)));
            SCE_QSCQ_WORD seen;
            if (!usable) {
                break;
            }
            seen = SCE_QSCQ_CAS(slot, entry, (SCE_QSCQ_WORD)(ticket_cycle ^ stored));
            if (seen == entry) {
                if (SCE_QSCQ_LOAD_SEQ(&r->threshold) != r->threshold_after_enqueue) {
                    SCE_QSCQ_STORE_SEQ(&r->threshold, r->threshold_after_enqueue);
                }
                return 1;
            }
            /* The entry changed; judge what it is now. */
            entry = seen;
        }
    }
}

/* Moves tail back to head after dequeues overshot an empty ring, so that the
 * next enqueue does not start a cycle ahead of what is read. `tail` is the word
 * as read, closing bit included, and the word written keeps the bit: a ring that
 * was closed stays closed. */
static inline void SCE_QSCQ_NAME(catch_up)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD tail, SCE_QSCQ_WORD head) {
    for (;;) {
        if (SCE_QSCQ_CAS(&r->tail, tail, (SCE_QSCQ_WORD)(head | (tail & SCE_QSCQ_FIN))) == tail) {
            return;
        }
        head = SCE_QSCQ_LOAD_ACQ(&r->head);
        tail = SCE_QSCQ_LOAD_ACQ(&r->tail);
        if (!SCE_QSCQ_NAME(before)((SCE_QSCQ_WORD)(tail & (SCE_QSCQ_WORD)~SCE_QSCQ_FIN), head)) {
            return;
        }
    }
}

/* Reads the entry a dequeue's ticket names until it is settled: nonzero, with
 * `*index` set, when the entry was this ticket's (the entry is left empty in the
 * same cycle); zero when the ticket found nothing and the caller goes on to the
 * empty check. */
static inline int SCE_QSCQ_NAME(look)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD *slot, SCE_QSCQ_WORD ticket_cycle,
                                      SCE_QSCQ_WORD *index) {
    unsigned looked = 0;
    for (;;) {
        SCE_QSCQ_WORD entry = SCE_QSCQ_LOAD_ACQ(slot);
        for (;;) {
            const SCE_QSCQ_WORD entry_cycle = (SCE_QSCQ_WORD)(entry | r->low_mask);
            SCE_QSCQ_WORD replacement;
            SCE_QSCQ_WORD seen;
            if (entry_cycle == ticket_cycle) {
                /* The entry is this ticket's: take its index and leave the entry
                 * empty in the same cycle. */
                SCE_QSCQ_FETCH_OR(slot, r->index_mask);
                *index = (SCE_QSCQ_WORD)(entry & r->index_mask);
                return 1;
            }
            if ((SCE_QSCQ_WORD)(entry | r->entry_count) != entry_cycle) {
                /* It holds an index of another cycle: mark it unsafe, so no
                 * enqueue of this cycle's ticket fills the entry behind the
                 * head. Already unsafe: nothing to do. */
                replacement = (SCE_QSCQ_WORD)(entry & (SCE_QSCQ_WORD)~r->entry_count);
                if (entry == replacement) {
                    return 0;
                }
            } else {
                /* It is empty. An enqueue holding this cycle's ticket may be
                 * about to fill it; look a few times, then move the entry to this
                 * cycle so that enqueue fails. */
                looked++;
                if (looked <= SCE_QSCQ_SPINS) {
                    break; /* look again from a fresh read */
                }
                replacement = (SCE_QSCQ_WORD)(ticket_cycle ^ ((SCE_QSCQ_WORD)~entry & r->entry_count));
            }
            if (!SCE_QSCQ_NAME(before)(entry_cycle, ticket_cycle)) {
                return 0;
            }
            seen = SCE_QSCQ_CAS(slot, entry, replacement);
            if (seen == entry) {
                return 0;
            }
            entry = seen;
        }
    }
}

/* The dequeue; `by_threshold` says whether the threshold may end it early. */
static inline int SCE_QSCQ_NAME(dequeue_by)(SCE_QSCQ_NAME(ring_t) * r, int by_threshold, SCE_QSCQ_WORD *index) {
    if (by_threshold && (SCE_QSCQ_SWORD)SCE_QSCQ_LOAD_SEQ(&r->threshold) < 0) {
        return 0;
    }
    for (;;) {
        const SCE_QSCQ_WORD head = SCE_QSCQ_FETCH_ADD(&r->head, 1);
        const SCE_QSCQ_WORD ticket_cycle = (SCE_QSCQ_WORD)((SCE_QSCQ_WORD)(head << 1) | r->low_mask);
        SCE_QSCQ_WORD tail;
        if (SCE_QSCQ_NAME(look)(r, &r->entries[SCE_QSCQ_NAME(map_ticket)(r, head)], ticket_cycle, index)) {
            return 1;
        }

        tail = SCE_QSCQ_LOAD_ACQ(&r->tail);
        if (SCE_QSCQ_NAME(at_or_before)((SCE_QSCQ_WORD)(tail & (SCE_QSCQ_WORD)~SCE_QSCQ_FIN),
                                        (SCE_QSCQ_WORD)(head + 1u))) {
            SCE_QSCQ_NAME(catch_up)(r, tail, (SCE_QSCQ_WORD)(head + 1u));
            if (by_threshold) {
                (void)SCE_QSCQ_FETCH_SUB(&r->threshold, 1);
            }
            return 0;
        }
        /* The old value was positive exactly when the new one is not below
         * zero. */
        if (by_threshold && (SCE_QSCQ_SWORD)SCE_QSCQ_FETCH_SUB(&r->threshold, 1) <= 0) {
            return 0;
        }
    }
}

/* Takes the oldest index off the ring: nonzero, with `*index` set, or zero when
 * the ring is empty. The empty answer may come from the threshold alone, which
 * does not look at a ticket an enqueue holds and has not yet filled. */
static inline int SCE_QSCQ_NAME(dequeue)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD *index) {
    return SCE_QSCQ_NAME(dequeue_by)(r, 1, index);
}

#if SCE_QSCQ_CLOSABLE
/* Takes the oldest index off a closed ring: nonzero, with `*index` set, or zero
 * once every ticket the ring ever granted has been consumed or made unusable. It
 * ignores the threshold and walks the head up to the tail, which a closed ring no
 * longer moves, so an enqueue still holding a ticket either finds its entry taken
 * from it (and takes the next ticket, which is refused) or has already filled it,
 * and then the walk returns its index. A segment is retired only after this
 * answers zero: until then an element may still arrive in it. */
static inline int SCE_QSCQ_NAME(dequeue_drained)(SCE_QSCQ_NAME(ring_t) * r, SCE_QSCQ_WORD *index) {
    return SCE_QSCQ_NAME(dequeue_by)(r, 0, index);
}
#endif

/* A bounded queue of at most `capacity` elements for any number of producers
 * and any number of consumers, lock-free; exactly `capacity` fit when nothing
 * else is running.
 *
 * The elements live in `capacity` slots of the caller's storage. Two rings of
 * indices make it a data queue: the free ring holds the indices of the slots
 * nobody is using and starts with all of them; the allocated ring holds the
 * indices of the slots that hold an element, in the order they were filled, and
 * starts empty. A push takes an index from the free ring (none: the queue is
 * full), writes its element into that slot and puts the index on the allocated
 * ring; a pop does the reverse. The rings carry only indices.
 *
 * A ring has `ring_slots` slots, a power of two at least `capacity` and at least
 * the most producers or consumers that will work the queue at once: the
 * algorithm's empty test is justified for at most that many enqueuers and
 * dequeuers on one ring (Nikolaev 2019, section 5.1). So a queue hands out at
 * most `ring_slots` producer places and as many consumer places.
 *
 * A push may be refused while others hold slots: a slot's index goes back to
 * the free ring only after the pop that took the element has read it, and a push
 * holds the index it took until it has published its element. Waiting for the
 * operation that holds the slot would not be lock-free, since that participant
 * may be stopped. The shortfall is at most one slot per other participant. */
typedef struct SCE_QSCQ_NAME(queue) {
    SCE_QSCQ_NAME(ring_t) allocated;
    SCE_QSCQ_NAME(ring_t) free_ring;
    uint32_t producers;
    uint32_t consumers;
    uint32_t capacity;
    uint32_t ring_slots;
} SCE_QSCQ_NAME(t);

/* Builds an empty queue. `allocated_entries` and `free_entries` each hold
 * 2 * ring_slots words. Returns zero, and builds nothing, for a capacity below
 * one, a ring size that is not a power of two, is below the capacity, or is more
 * than a ring of this entry width can number. */
static inline int SCE_QSCQ_NAME(init)(SCE_QSCQ_NAME(t) * q, uint32_t capacity, uint32_t ring_slots,
                                      SCE_QSCQ_WORD *allocated_entries, SCE_QSCQ_WORD *free_entries) {
    if (capacity < 1u || ring_slots < 1u || (ring_slots & (ring_slots - 1u)) != 0u || ring_slots < capacity ||
        ring_slots > (uint32_t)SCE_QSCQ_MAX_RING_SLOTS) {
        return 0;
    }
    SCE_QSCQ_NAME(ring_init)(&q->allocated, allocated_entries, (SCE_QSCQ_WORD)ring_slots, 0);
    SCE_QSCQ_NAME(ring_init)(&q->free_ring, free_entries, (SCE_QSCQ_WORD)ring_slots, (SCE_QSCQ_WORD)capacity);
    q->producers = 0;
    q->consumers = 0;
    q->capacity = capacity;
    q->ring_slots = ring_slots;
    return 1;
}

/* Takes one of the producer places, or returns zero when every one is taken:
 * the ring cannot be shared by more enqueuers than it has slots. */
static inline int SCE_QSCQ_NAME(producer_acquire)(SCE_QSCQ_NAME(t) * q) {
    return sce_queue_take_place(&q->producers, q->ring_slots);
}

static inline void SCE_QSCQ_NAME(producer_release)(SCE_QSCQ_NAME(t) * q) {
    (void)sce_atomic_fetch_sub_acq_rel_u32(&q->producers, 1u);
}

static inline int SCE_QSCQ_NAME(consumer_acquire)(SCE_QSCQ_NAME(t) * q) {
    return sce_queue_take_place(&q->consumers, q->ring_slots);
}

static inline void SCE_QSCQ_NAME(consumer_release)(SCE_QSCQ_NAME(t) * q) {
    (void)sce_atomic_fetch_sub_acq_rel_u32(&q->consumers, 1u);
}

/* Takes `*value` (element_size bytes) into the queue, or reports full when the
 * queue holds its capacity or other participants' operations hold the slots it
 * is short of. Lock-free: some operation completes in a bounded number of steps
 * whatever the other participants are doing. */
static inline sce_queue_push_status_t SCE_QSCQ_NAME(try_push)(SCE_QSCQ_NAME(t) * q, unsigned char *slots,
                                                              size_t element_size, const void *value) {
    SCE_QSCQ_WORD index;
    if (!SCE_QSCQ_NAME(dequeue)(&q->free_ring, &index)) {
        return SCE_QUEUE_PUSH_FULL;
    }
    /* The index came off the free ring, so no other participant holds this slot
     * until the enqueue below hands it on. */
    memcpy(slots + (size_t)index * element_size, value, element_size);
    /* A queue built by `init` is never closed, so the put cannot be refused. */
    (void)SCE_QSCQ_NAME(enqueue)(&q->allocated, index);
    return SCE_QUEUE_PUSH_OK;
}

#if SCE_QSCQ_CLOSABLE
/* Takes `*value` into the queue and returns nonzero, or returns zero and leaves
 * the value with the caller: when the queue has no free slot it is closed, and
 * when it is closed already nothing more is ever pushed into it, which is what
 * lets a segment list put every later element behind this segment's. The element
 * is copied in, so the caller's value is untouched either way. Lock-free. */
static inline int SCE_QSCQ_NAME(push_or_close)(SCE_QSCQ_NAME(t) * q, unsigned char *slots, size_t element_size,
                                               const void *value) {
    SCE_QSCQ_WORD index;
    if (!SCE_QSCQ_NAME(dequeue)(&q->free_ring, &index)) {
        SCE_QSCQ_NAME(ring_close)(&q->allocated);
        return 0;
    }
    memcpy(slots + (size_t)index * element_size, value, element_size);
    if (SCE_QSCQ_NAME(enqueue)(&q->allocated, index)) {
        return 1;
    }
    /* The ring was closed under the push: the index is still ours, and goes back
     * to the free ring, which is never closed. */
    (void)SCE_QSCQ_NAME(enqueue)(&q->free_ring, index);
    return 0;
}

/* Takes the oldest element of a closed queue into `*out`, or returns zero only
 * when no element can ever arrive in it again (`dequeue_drained`). Lock-free. */
static inline int SCE_QSCQ_NAME(pop_drained)(SCE_QSCQ_NAME(t) * q, unsigned char *slots, size_t element_size,
                                             void *out) {
    SCE_QSCQ_WORD index;
    if (!SCE_QSCQ_NAME(dequeue_drained)(&q->allocated, &index)) {
        return 0;
    }
    memcpy(out, slots + (size_t)index * element_size, element_size);
    (void)SCE_QSCQ_NAME(enqueue)(&q->free_ring, index);
    return 1;
}
#endif

/* Takes the oldest element into `*out`, or returns zero when the queue is
 * empty. Lock-free. */
static inline int SCE_QSCQ_NAME(try_pop)(SCE_QSCQ_NAME(t) * q, unsigned char *slots, size_t element_size, void *out) {
    SCE_QSCQ_WORD index;
    if (!SCE_QSCQ_NAME(dequeue)(&q->allocated, &index)) {
        return 0;
    }
    /* The index came off the allocated ring, so the slot holds a pushed element
     * and no other participant holds the slot until the enqueue below hands it
     * on. */
    memcpy(out, slots + (size_t)index * element_size, element_size);
    (void)SCE_QSCQ_NAME(enqueue)(&q->free_ring, index);
    return 1;
}

#undef SCE_QSCQ_CLOSABLE
#undef SCE_QSCQ_FIN
#undef SCE_QSCQ_WORD
#undef SCE_QSCQ_SWORD
#undef SCE_QSCQ_BITS
#undef SCE_QSCQ_MAX_RING_SLOTS
#undef SCE_QSCQ_PREFIX
#undef SCE_QSCQ_LOAD_ACQ
#undef SCE_QSCQ_LOAD_SEQ
#undef SCE_QSCQ_STORE_SEQ
#undef SCE_QSCQ_CAS
#undef SCE_QSCQ_FETCH_ADD
#undef SCE_QSCQ_FETCH_SUB
#undef SCE_QSCQ_FETCH_OR
#undef SCE_QSCQ_LINE_SHIFT
#undef SCE_QSCQ_SPINS
#undef SCE_QSCQ_NAME
#undef SCE_QSCQ_PASTE
#undef SCE_QSCQ_PASTE_
