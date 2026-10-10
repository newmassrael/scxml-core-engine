/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the `segmented` storage mode of `sce:kind="queue"` (SCE
 * Protocol-Synthesis RFC §synth-5-P) for C11: queues that grow by segments taken
 * from an allocator the caller injects, and hold as many elements as that
 * allocator allows.
 *
 *   - sce_queue_linked_*   one producer, one consumer: linked Lamport rings. Pop is
 *                          wait-free; push is wait-free but for the allocator.
 *   - sce_queue_lscq_*     any other cardinality: Nikolaev's LSCQ (DISC 2019), a
 *                          list of SCQ rings reclaimed through a hazard-pointer
 *                          domain. Both operations are lock-free but for the
 *                          allocator. It needs 64-bit atomics: a ring is closed by
 *                          the top bit of its tail (queue_scq_width.inc).
 *   - sce_hazard_*         the reclamation domain the LSCQ needs, sized from the
 *                          number of participants and built by the caller.
 *
 * There is no global default allocator and no global domain (SCE_FORGE.md §2.1,
 * C2): the queue borrows both, so it must not move while it lives, and a target
 * with no heap hands it an arena. Nothing here calls `malloc`.
 *
 * What differs from the languages with generics and destructors:
 *
 *   - An element is a plain value copied in and out by memcpy, the generated codec
 *     struct, with no destructor to run. A queue destroyed with elements in it
 *     gives its segments back and runs nothing for them.
 *   - The allocator is a struct of two function pointers and a context, and
 *     states the progress its operations give. The queue holds it to the
 *     progress the document declared (`allocator-progress`) when it is built and
 *     refuses the construction when the allocator gives less; C has no
 *     compile-time check to make (RFC §synth-5-P, *Segment allocator progress is
 *     declared in the document*).
 *   - A pointer lives in a `size_t` word and is converted at the edges, because
 *     the atomics (sce/forge/atomics.h, the RFC §synth-5-I family) have no
 *     pointer type. The word is the platform's `usize`.
 *   - A handle on a side is a place (a count) and, for the LSCQ, a slot of the
 *     hazard domain; the caller keeps the slot's index and passes it to each
 *     operation.
 */

#ifndef SCE_FORGE_QUEUE_SEGMENTED_H
#define SCE_FORGE_QUEUE_SEGMENTED_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <sce/forge/queue.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A pointer is held in a `size_t` word; it must fit. */
typedef char sce_queue_pointer_fits_in_size_t[(sizeof(size_t) == sizeof(void *)) ? 1 : -1];

static inline size_t sce_queue_word_of(const void *pointer) {
    return (size_t)(uintptr_t)pointer;
}

static inline void *sce_queue_pointer_of(size_t word) {
    return (void *)(uintptr_t)word;
}

/* What an operation guarantees about how long it takes whatever the other
 * participants are doing, ordered by strength: a stronger guarantee compares
 * greater. */
typedef enum sce_queue_progress {
    SCE_QUEUE_PROGRESS_BLOCKING = 0,
    SCE_QUEUE_PROGRESS_LOCK_FREE = 1,
    SCE_QUEUE_PROGRESS_WAIT_FREE = 2
} sce_queue_progress_t;

/* The allocator a segmented queue is injected with.
 *
 *   allocate    a block of `size` bytes aligned to `align` (a power of two) that
 *               no other live block overlaps, or NULL when it refuses; a refusal
 *               is the queue's SCE_QUEUE_PUSH_OUT_OF_MEMORY.
 *   deallocate  gives a block back, with the size and alignment it was given.
 *   progress    what allocate and deallocate give; the queue's push is no
 *               stronger than this.
 *
 * Both functions may be called from several threads at once. */
typedef struct sce_queue_allocator {
    void *context;
    void *(*allocate)(void *context, size_t size, size_t align);
    void (*deallocate)(void *context, void *block, size_t size, size_t align);
    sce_queue_progress_t progress;
} sce_queue_allocator_t;

/* Whether `allocator` is usable and gives at least `required`. */
static inline int sce_queue_allocator_gives(const sce_queue_allocator_t *allocator, sce_queue_progress_t required) {
    return allocator != NULL && allocator->allocate != NULL && allocator->deallocate != NULL &&
           allocator->progress >= required;
}

static inline size_t sce_queue_align_up(size_t value, size_t align) {
    return (value + align - 1u) & ~(align - 1u);
}

static inline int sce_queue_is_power_of_two(size_t value) {
    return value != 0u && (value & (value - 1u)) == 0u;
}

/* ------------------------------------------------------------------------ */
/* Hazard pointers                                                            */
/* ------------------------------------------------------------------------ */

/* The reclamation domain of a segmented queue (RFC §synth-5-P, *Reclamation
 * domain*): hazard pointers, sized statically from the number of participants
 * and built by the caller.
 *
 * A segment can still be read by one participant after another has taken it out
 * of the queue, so it cannot be freed at that moment. Each participant publishes,
 * in a slot of its own, the one segment it is about to touch (sce_hazard_protect);
 * the thread that unlinks a segment hands it to the domain (sce_hazard_retire);
 * and the domain frees it only when no slot names it. A participant that stalls
 * therefore holds back the segments its slot names and nothing else, which is
 * why hazard pointers were chosen over epochs: the memory a stalled participant
 * keeps unreclaimed is bounded, so the domain needs no storage that grows.
 *
 * The domain is a fixed array of slots the caller provides, and the retired
 * segments wait on an intrusive list threaded through the header each carries, so
 * it needs neither a heap nor a bound on the list.
 *
 * Progress. protect and retire are lock-free: a retry means another participant
 * moved the pointer or pushed a node. The scan that frees nodes is run by one
 * thread at a time; a thread that finds a scan running does not wait for it, it
 * returns and leaves the nodes on the list.
 *
 * Ordering. A participant publishes a hazard with a sequentially consistent
 * store, then a sequentially consistent fence, then re-validates with a
 * sequentially consistent load of the pointer it guards; whoever unlinks a node
 * puts a sequentially consistent fence after the unlinking compare-and-swaps and
 * before the retirement, and the scan puts one before it reads the hazards. Either
 * the scan sees the hazard and keeps the node, or the participant's validation
 * sees the pointer already moved and does not use it. (The family has no
 * sequentially consistent compare-and-swap, so the fences are what give the
 * unlinking compare-and-swaps the order that argument needs.) That holds when the
 * location a participant validates against is written by whoever unlinks the node
 * before the node is retired; the queue using the domain has to arrange that
 * (sce_queue_lscq_try_pop, *Where a hazard is validated*). */

#define SCE_HAZARD_CACHE_LINE 64u

typedef struct sce_hazard_slot {
    /* The segment the slot's participant is touching, as a pointer word, or 0. */
    size_t pointer;
    /* 1 while a participant holds the slot. */
    uint32_t taken;
    unsigned char pad[SCE_HAZARD_CACHE_LINE - sizeof(size_t) - sizeof(uint32_t)];
} sce_hazard_slot_t;

/* The header a retired object carries so the domain can chain it and, once no
 * hazard names it, hand it back to what allocated it. It is the first member of
 * the object: a hazard names the object by its address, and the scan compares
 * that address with the header's. */
typedef struct sce_hazard_retired {
    /* The next retired object on the domain's list, as a pointer word. Written
     * only by the domain. */
    size_t chain;
    /* What frees the object; written by sce_hazard_retire before the object is
     * published to the list, read by the scan after it took the list. */
    void (*reclaim)(struct sce_hazard_retired *node, void *context);
    /* The value `reclaim` is called with. */
    void *context;
} sce_hazard_retired_t;

typedef struct sce_hazard_domain {
    sce_hazard_slot_t *slots;
    uint32_t slot_count;
    /* Retired objects not yet freed, as a pointer word. */
    size_t retired;
    /* How many are on `retired`; a scan runs when twice the slots are waiting, so
     * the unreclaimed memory is bounded by the participants. */
    size_t waiting;
    /* 1 while a scan runs. */
    uint32_t scanning;
} sce_hazard_domain_t;

/* Builds an empty domain over `count` slots at `slots`, which it borrows. Returns
 * zero, and builds nothing, for no slot. */
static inline int sce_hazard_domain_init(sce_hazard_domain_t *domain, sce_hazard_slot_t *slots, uint32_t count) {
    uint32_t index;
    if (domain == NULL || slots == NULL || count < 1u) {
        return 0;
    }
    for (index = 0; index < count; index++) {
        slots[index].pointer = 0u;
        slots[index].taken = 0u;
    }
    domain->slots = slots;
    domain->slot_count = count;
    domain->retired = 0u;
    domain->waiting = 0u;
    domain->scanning = 0u;
    return 1;
}

/* A slot for one participant: its index, or -1 when all are taken. */
static inline int sce_hazard_acquire(sce_hazard_domain_t *domain) {
    uint32_t index;
    for (index = 0; index < domain->slot_count; index++) {
        if (sce_atomic_cas_strong_acq_rel_u32(&domain->slots[index].taken, 0u, 1u) == 0u) {
            return (int)index;
        }
    }
    return -1;
}

/* Stops protecting what slot `index` names, so it does not hold back the segment
 * longer than its participant uses it. */
static inline void sce_hazard_clear(sce_hazard_domain_t *domain, uint32_t index) {
    sce_atomic_store_release_usize(&domain->slots[index].pointer, 0u);
}

/* Gives slot `index` back. */
static inline void sce_hazard_release(sce_hazard_domain_t *domain, uint32_t index) {
    sce_hazard_clear(domain, index);
    sce_atomic_store_release_u32(&domain->slots[index].taken, 0u);
}

/* Reads `*source` and protects what it points to: publishes it, then checks that
 * `*source` still holds it, and tries again if not. The pointer word returned is
 * not freed while the slot names it, unless it is 0. */
static inline size_t sce_hazard_protect(sce_hazard_domain_t *domain, uint32_t index, const size_t *source) {
    size_t pointer = sce_atomic_load_acquire_usize(source);
    for (;;) {
        size_t again;
        sce_atomic_store_seq_cst_usize(&domain->slots[index].pointer, pointer);
        sce_atomic_fence_seq_cst();
        again = sce_atomic_load_seq_cst_usize(source);
        if (again == pointer) {
            return pointer;
        }
        pointer = again;
    }
}

/* Pushes the chain `first` .. `last` (linked through `chain`) onto the list. */
static inline void sce_hazard_push(sce_hazard_domain_t *domain, sce_hazard_retired_t *first,
                                   sce_hazard_retired_t *last) {
    size_t head = sce_atomic_load_acquire_usize(&domain->retired);
    for (;;) {
        size_t seen;
        sce_atomic_store_release_usize(&last->chain, head);
        seen = sce_atomic_cas_strong_acq_rel_usize(&domain->retired, head, sce_queue_word_of(first));
        if (seen == head) {
            return;
        }
        head = seen;
    }
}

/* Whether some slot names the object at `word`. */
static inline int sce_hazard_is_named(const sce_hazard_domain_t *domain, size_t word) {
    uint32_t index;
    for (index = 0; index < domain->slot_count; index++) {
        if (sce_atomic_load_seq_cst_usize(&domain->slots[index].pointer) == word) {
            return 1;
        }
    }
    return 0;
}

/* The scan, run by the thread that holds `scanning`; it gives the flag back. */
static inline void sce_hazard_scan_held(sce_hazard_domain_t *domain) {
    sce_hazard_retired_t *list =
        (sce_hazard_retired_t *)sce_queue_pointer_of(sce_atomic_xchg_acq_rel_usize(&domain->retired, 0u));
    sce_hazard_retired_t *kept = NULL;
    sce_hazard_retired_t *kept_last = NULL;
    size_t freed = 0u;
    /* The objects on `list` were unlinked before they were retired; the fence puts
     * that unlinking before the reads of the hazards below, so a participant that
     * has not published a hazard for one by now will find the pointer moved when it
     * validates. */
    sce_atomic_fence_seq_cst();
    while (list != NULL) {
        sce_hazard_retired_t *node = list;
        list = (sce_hazard_retired_t *)sce_queue_pointer_of(sce_atomic_load_acquire_usize(&node->chain));
        if (sce_hazard_is_named(domain, sce_queue_word_of(node))) {
            sce_atomic_store_release_usize(&node->chain, sce_queue_word_of(kept));
            if (kept == NULL) {
                kept_last = node;
            }
            kept = node;
        } else {
            /* No hazard names it, nothing else reaches it, and the retirement
             * recorded how to free it. */
            node->reclaim(node, node->context);
            freed++;
        }
    }
    if (kept != NULL) {
        sce_hazard_push(domain, kept, kept_last);
    }
    if (freed > 0u) {
        (void)sce_atomic_fetch_sub_acq_rel_usize(&domain->waiting, freed);
    }
    sce_atomic_store_release_u32(&domain->scanning, 0u);
}

/* Frees what can be freed now: the retired objects no hazard names, unless a scan
 * is already running, which is then left to finish. Lock-free: it never waits for
 * another scan. */
static inline void sce_hazard_collect(sce_hazard_domain_t *domain) {
    if (sce_atomic_xchg_acq_rel_u32(&domain->scanning, 1u) != 0u) {
        return;
    }
    sce_hazard_scan_held(domain);
}

/* Frees every retired object no hazard names, waiting for a scan that is running
 * instead of leaving the objects to it. A queue being destroyed calls it: no
 * participant of that queue is alive, so none of its segments is named, and every
 * one of them is freed before the queue's allocator can go. Blocking, which a
 * destroy may be. */
static inline void sce_hazard_flush(sce_hazard_domain_t *domain) {
    while (sce_atomic_xchg_acq_rel_u32(&domain->scanning, 1u) != 0u) {
        /* Another thread is scanning; it holds the flag only for one pass. */
    }
    sce_hazard_scan_held(domain);
}

/* Hands `node`, already unlinked from every structure a participant can reach it
 * through, to the domain. It is freed by `reclaim(node, context)` once no hazard
 * names it, now or at a later retirement. `node` is the header of a live object
 * no longer reachable except through a hazard already published; it is retired
 * once; `reclaim` frees exactly that object; and `context` stays valid until it
 * has. */
static inline void sce_hazard_retire(sce_hazard_domain_t *domain, sce_hazard_retired_t *node,
                                     void (*reclaim)(sce_hazard_retired_t *node, void *context), void *context) {
    size_t waiting;
    node->reclaim = reclaim;
    node->context = context;
    /* Counted before it is published, so a scan that frees it at once never
     * subtracts what was not yet added. */
    waiting = sce_atomic_fetch_add_acq_rel_usize(&domain->waiting, 1u) + 1u;
    sce_hazard_push(domain, node, node);
    if (waiting >= 2u * (size_t)domain->slot_count) {
        sce_hazard_collect(domain);
    }
}

/* How many retired objects are waiting to be freed. */
static inline size_t sce_hazard_waiting(const sce_hazard_domain_t *domain) {
    return sce_atomic_load_acquire_usize(&domain->waiting);
}

/* ------------------------------------------------------------------------ */
/* Linked Lamport rings                                                       */
/* ------------------------------------------------------------------------ */

/* The `segmented` row for one producer and one consumer (RFC §synth-5-P): linked
 * Lamport rings, wait-free on both sides, bounded only by the allocator.
 *
 * The queue is a list of segments. A segment holds `segment_len` slots that are
 * filled once, in order, and read once, in order: it is a Lamport ring that is
 * never lapped, so it needs neither the second lap that tells a full ring from an
 * empty one nor a free slot. The producer fills the newest segment and, when it
 * is full, takes a new one from the allocator and links it behind; the consumer
 * reads the oldest segment and, when it has read them all, follows the link and
 * gives the segment back. Each side keeps its own position and shares it with the
 * other only through the two things the other must see: how many slots of a
 * segment are filled, and the link to the next segment.
 *
 * No reclamation domain. The producer never touches a segment again after it has
 * linked its successor, and the consumer gives a segment back only after it has
 * read that link, so the consumer frees it directly.
 *
 * Progress. Pop is wait-free. Push is wait-free but for the one step that can
 * take a segment: the allocator. A push the allocator refuses leaves its element
 * with the caller and reports SCE_QUEUE_PUSH_OUT_OF_MEMORY.
 *
 * Ordering. The producer publishes a filled slot by storing the count of filled
 * slots with release, and the consumer reads that count with acquire before it
 * reads the slot, so the write of an element happens-before the pop that returns
 * it. The link is stored with release and read with acquire for the same reason.
 * Each side's own position is private to it and needs no ordering of its own. */

typedef struct sce_queue_linked_segment {
    /* How many slots, from the first, hold an element the producer published.
     * Written only by the producer. */
    size_t written;
    /* The segment after this one, as a pointer word, or 0 while this is the
     * newest. Written once, by the producer. */
    size_t next;
} sce_queue_linked_segment_t;

typedef struct sce_queue_linked {
    sce_queue_allocator_t allocator;
    size_t element_size;
    size_t slots_offset;
    size_t segment_bytes;
    size_t block_align;
    size_t segment_len;
    /* The segment the consumer reads, and how many of its elements it has taken.
     * Private to the consumer. */
    sce_queue_linked_segment_t *head;
    size_t head_read;
    /* The segment the producer fills. Private to the producer. */
    sce_queue_linked_segment_t *tail;
    /* 1 while the producer / consumer handle is taken: there is one of each. */
    uint32_t producer_taken;
    uint32_t consumer_taken;
} sce_queue_linked_t;

static inline unsigned char *sce_queue_linked_slot(const sce_queue_linked_t *q, sce_queue_linked_segment_t *segment,
                                                   size_t index) {
    return (unsigned char *)segment + q->slots_offset + index * q->element_size;
}

static inline sce_queue_linked_segment_t *sce_queue_linked_new_segment(sce_queue_linked_t *q) {
    sce_queue_linked_segment_t *segment =
        (sce_queue_linked_segment_t *)q->allocator.allocate(q->allocator.context, q->segment_bytes, q->block_align);
    if (segment != NULL) {
        segment->written = 0u;
        segment->next = 0u;
    }
    return segment;
}

static inline void sce_queue_linked_free_segment(sce_queue_linked_t *q, sce_queue_linked_segment_t *segment) {
    q->allocator.deallocate(q->allocator.context, segment, q->segment_bytes, q->block_align);
}

/* Builds an empty queue of `segment_len`-element segments of `element_size`
 * bytes, each element aligned to `element_align`, over `*allocator` (copied). The
 * allocator must give at least `required`, the progress the document declared for
 * it. Returns zero, and builds nothing, when it gives less, when a size is zero or
 * an alignment is not a power of two, or when the allocator refuses the first
 * segment. */
static inline int sce_queue_linked_init(sce_queue_linked_t *q, const sce_queue_allocator_t *allocator,
                                        sce_queue_progress_t required, size_t element_size, size_t element_align,
                                        size_t segment_len) {
    size_t header_align = sizeof(size_t);
    if (q == NULL || !sce_queue_allocator_gives(allocator, required) || element_size < 1u || segment_len < 1u ||
        !sce_queue_is_power_of_two(element_align) || segment_len > ((size_t)-1) / element_size) {
        return 0;
    }
    q->allocator = *allocator;
    q->element_size = element_size;
    q->slots_offset = sce_queue_align_up(sizeof(sce_queue_linked_segment_t), element_align);
    q->block_align = element_align > header_align ? element_align : header_align;
    q->segment_len = segment_len;
    if (segment_len * element_size > ((size_t)-1) - q->slots_offset) {
        return 0;
    }
    q->segment_bytes = q->slots_offset + segment_len * element_size;
    q->producer_taken = 0u;
    q->consumer_taken = 0u;
    q->head_read = 0u;
    q->head = sce_queue_linked_new_segment(q);
    q->tail = q->head;
    return q->head != NULL;
}

/* Destroys the queue: gives every segment back. The elements still in it are
 * plain values and need nothing run. No handle may be alive. */
static inline void sce_queue_linked_destroy(sce_queue_linked_t *q) {
    sce_queue_linked_segment_t *segment = q->head;
    while (segment != NULL) {
        sce_queue_linked_segment_t *next =
            (sce_queue_linked_segment_t *)sce_queue_pointer_of(sce_atomic_load_acquire_usize(&segment->next));
        sce_queue_linked_free_segment(q, segment);
        segment = next;
    }
    q->head = NULL;
    q->tail = NULL;
}

/* Takes the producer place, or returns zero when it is taken. */
static inline int sce_queue_linked_producer_acquire(sce_queue_linked_t *q) {
    return sce_queue_take_place(&q->producer_taken, 1u);
}

static inline void sce_queue_linked_producer_release(sce_queue_linked_t *q) {
    sce_atomic_store_release_u32(&q->producer_taken, 0u);
}

/* Takes the consumer place, or returns zero when it is taken. */
static inline int sce_queue_linked_consumer_acquire(sce_queue_linked_t *q) {
    return sce_queue_take_place(&q->consumer_taken, 1u);
}

static inline void sce_queue_linked_consumer_release(sce_queue_linked_t *q) {
    sce_atomic_store_release_u32(&q->consumer_taken, 0u);
}

/* Takes a copy of `*value` into the queue, or reports SCE_QUEUE_PUSH_OUT_OF_MEMORY
 * when the newest segment is full and the allocator will not give another; the
 * value is the caller's either way. Wait-free when the allocator is. */
static inline sce_queue_push_status_t sce_queue_linked_try_push(sce_queue_linked_t *q, const void *value) {
    sce_queue_linked_segment_t *tail = q->tail;
    /* Only the producer writes the count, so it reads its own. */
    size_t written = tail->written;
    if (written == q->segment_len) {
        sce_queue_linked_segment_t *fresh = sce_queue_linked_new_segment(q);
        if (fresh == NULL) {
            return SCE_QUEUE_PUSH_OUT_OF_MEMORY;
        }
        /* The producer never touches `tail` again after this store. */
        sce_atomic_store_release_usize(&tail->next, sce_queue_word_of(fresh));
        q->tail = fresh;
        tail = fresh;
        written = 0u;
    }
    /* Slot `written` is past every filled slot, so the consumer does not read it
     * until the store below publishes it. */
    memcpy(sce_queue_linked_slot(q, tail, written), value, q->element_size);
    sce_atomic_store_release_usize(&tail->written, written + 1u);
    return SCE_QUEUE_PUSH_OK;
}

/* Takes the oldest element into `*out`, or returns zero when the queue is empty.
 * Wait-free. */
static inline int sce_queue_linked_try_pop(sce_queue_linked_t *q, void *out) {
    sce_queue_linked_segment_t *head = q->head;
    size_t read = q->head_read;
    size_t written;
    if (read == q->segment_len) {
        /* Every element of the head segment is taken. Its successor exists once the
         * producer has linked it, and the producer never touches this segment after
         * that, so it is ours to give back. */
        sce_queue_linked_segment_t *next =
            (sce_queue_linked_segment_t *)sce_queue_pointer_of(sce_atomic_load_acquire_usize(&head->next));
        if (next == NULL) {
            return 0;
        }
        sce_queue_linked_free_segment(q, head);
        head = next;
        read = 0u;
        q->head = head;
        q->head_read = 0u;
    }
    written = sce_atomic_load_acquire_usize(&head->written);
    if (read >= written) {
        return 0;
    }
    memcpy(out, sce_queue_linked_slot(q, head, read), q->element_size);
    q->head_read = read + 1u;
    return 1;
}

/* ------------------------------------------------------------------------ */
/* LSCQ                                                                       */
/* ------------------------------------------------------------------------ */

/* The `segmented` row for any cardinality other than one producer and one
 * consumer (RFC §synth-5-P): LSCQ, a list of SCQ rings, lock-free on both sides
 * and bounded only by the allocator.
 *
 * The algorithm is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott list
 * whose nodes are SCQ rings instead of single elements. Producers push into the
 * newest ring (`tail`); consumers pop from the oldest (`head`). A ring that cannot
 * take another element is closed: nothing is ever pushed into it again. The
 * producer that found it closed takes a segment from the allocator, puts its
 * element in it before anyone can see it, and links it behind the closed one with
 * a compare-and-swap on that ring's `next`; a producer that loses that race frees
 * its segment and pushes into the winner's. A consumer that finds the oldest ring
 * empty and closed moves `head` to its successor and retires it.
 *
 * Each ring is the 64-bit SCQ data queue of queue.h, which keeps its elements in
 * a slot array and moves their indices through two rings; a segment is allocated
 * whole, the rings' entry arrays and the slots in the same block.
 *
 * Closing. The close is a bit in the allocated ring's tail, set with a
 * fetch-and-or, and an enqueue takes its ticket with a fetch-and-add that returns
 * the bit: a ticket granted before the close can still be used, one after it is
 * refused. That is what makes the order across segments exact.
 *
 * Retiring a ring waits for the elements in flight. A consumer sees a ring empty
 * through SCQ's threshold, which does not look at a ticket an enqueue holds and
 * has not yet filled. So the consumer that finds the oldest ring empty and closed
 * pops it once more with sce_queue_scq64_pop_drained, which walks the head to the
 * closed tail and either takes the element such an enqueue filled or makes its
 * entry unusable, and only when that answers empty does it move `head`.
 *
 * Reclamation. A segment is freed through the hazard-pointer domain the caller
 * builds; each handle holds one slot of it and protects the one segment it is
 * touching. A segment is retired only by the consumer whose compare-and-swap moved
 * `head` past it.
 *
 * Where a hazard is validated. A hazard protects a segment only if the location it
 * is validated against is the one the unlinking thread writes before the scan
 * runs. A consumer validates against `head`, which the retiring consumer writes. A
 * producer validates against `tail`, which that consumer does not write: left
 * alone, `tail` can still name a segment that `head` has left and that has been
 * retired (the producer that linked its successor has not yet moved `tail`), so a
 * producer could publish its hazard after the scan read its slot, find `tail`
 * unchanged, and use the segment once the scan freed it. So a consumer moves `tail`
 * off the segment, to its successor, before it moves `head` past it, and puts a
 * sequentially consistent fence after both before it retires the segment. Then
 * `tail` does not name a segment by the time it is retired, and any producer whose
 * check still saw it there published its hazard before the retirement, which the
 * scan reads after it.
 *
 * Progress. Push and pop are lock-free: a retry means another participant moved
 * `head`, `tail` or a ring, and the helping step lets a push finish what a stalled
 * one started. A push is as strong as the allocator it calls; a refusal leaves the
 * element with the caller and reports SCE_QUEUE_PUSH_OUT_OF_MEMORY.
 *
 * A ring of `ring_slots` slots is correct for at most that many producers and as
 * many consumers at once (Nikolaev 2019, section 5.1), so the queue hands out at
 * most that many places a side, and the domain has a slot for every handle. */

typedef struct sce_queue_lscq_segment {
    /* The retirement header, first: the domain names a segment by it. */
    sce_hazard_retired_t retired;
    /* The segment after this one, as a pointer word, or 0 while this is the
     * newest. Written once, by the producer whose compare-and-swap links a
     * successor. */
    size_t next;
    sce_queue_scq64_t ring;
} sce_queue_lscq_segment_t;

typedef struct sce_queue_lscq {
    sce_queue_allocator_t allocator;
    sce_hazard_domain_t *domain;
    /* The oldest segment, and the newest one (or the one before it while a
     * producer is between linking a successor and moving `tail`), as pointer
     * words. */
    size_t head;
    size_t tail;
    uint32_t producers;
    uint32_t consumers;
    uint32_t places;
    uint32_t segment_len;
    uint32_t ring_slots;
    size_t element_size;
    size_t allocated_offset;
    size_t free_offset;
    size_t slots_offset;
    size_t segment_bytes;
    size_t block_align;
} sce_queue_lscq_t;

static inline unsigned char *sce_queue_lscq_slots(const sce_queue_lscq_t *q, sce_queue_lscq_segment_t *segment) {
    return (unsigned char *)segment + q->slots_offset;
}

/* A segment with an open, empty ring, or NULL when the allocator refuses. */
static inline sce_queue_lscq_segment_t *sce_queue_lscq_new_segment(sce_queue_lscq_t *q) {
    sce_queue_lscq_segment_t *segment =
        (sce_queue_lscq_segment_t *)q->allocator.allocate(q->allocator.context, q->segment_bytes, q->block_align);
    if (segment == NULL) {
        return NULL;
    }
    segment->retired.chain = 0u;
    segment->retired.reclaim = NULL;
    segment->retired.context = NULL;
    segment->next = 0u;
    /* The sizes were checked when the queue was built, so this does not refuse. */
    (void)sce_queue_scq64_init(&segment->ring, q->segment_len, q->ring_slots,
                               (uint64_t *)((unsigned char *)segment + q->allocated_offset),
                               (uint64_t *)((unsigned char *)segment + q->free_offset));
    return segment;
}

static inline void sce_queue_lscq_free_segment(sce_queue_lscq_t *q, sce_queue_lscq_segment_t *segment) {
    q->allocator.deallocate(q->allocator.context, segment, q->segment_bytes, q->block_align);
}

/* What the domain calls once no hazard names a retired segment: `context` is the
 * queue, which does not move while it lives. */
static inline void sce_queue_lscq_reclaim(sce_hazard_retired_t *node, void *context) {
    sce_queue_lscq_free_segment((sce_queue_lscq_t *)context, (sce_queue_lscq_segment_t *)node);
}

/* Builds an empty queue of `segment_len`-element segments of `element_size` bytes,
 * each element aligned to `element_align`, whose rings have `ring_slots` slots (a
 * power of two, at least `segment_len` and at least the most producers or consumers
 * at once), over `*allocator` (copied) and `*domain`, which it borrows. The
 * allocator must give at least `required`, the progress the document declared for
 * it. Returns zero, and builds nothing, when it gives less, when a size or ring
 * size is one the rings refuse, or when the allocator refuses the first segment. */
static inline int sce_queue_lscq_init(sce_queue_lscq_t *q, sce_hazard_domain_t *domain,
                                      const sce_queue_allocator_t *allocator, sce_queue_progress_t required,
                                      size_t element_size, size_t element_align, uint32_t segment_len,
                                      uint32_t ring_slots) {
    sce_queue_lscq_segment_t *first;
    size_t entries_bytes;
    size_t block_align = sizeof(size_t) > sizeof(uint64_t) ? sizeof(size_t) : sizeof(uint64_t);
    if (q == NULL || domain == NULL || !sce_queue_allocator_gives(allocator, required) || element_size < 1u ||
        segment_len < 1u || !sce_queue_is_power_of_two(element_align) || ring_slots < segment_len ||
        !sce_queue_is_power_of_two(ring_slots) || ring_slots > (uint32_t)(1u << 30) ||
        element_size > ((size_t)-1) / (size_t)segment_len / 2u) {
        return 0;
    }
    /* The entry arrays of the two rings: refused when the byte count does not fit a
     * `size_t`, which a 32-bit one can overflow with a large ring. */
    entries_bytes = 2u * (size_t)ring_slots * sizeof(uint64_t);
    if (entries_bytes / (2u * sizeof(uint64_t)) != (size_t)ring_slots) {
        return 0;
    }
    q->allocator = *allocator;
    q->domain = domain;
    q->element_size = element_size;
    q->segment_len = segment_len;
    q->ring_slots = ring_slots;
    q->places = ring_slots;
    q->allocated_offset = sce_queue_align_up(sizeof(sce_queue_lscq_segment_t), sizeof(uint64_t));
    q->free_offset = q->allocated_offset + entries_bytes;
    q->slots_offset = sce_queue_align_up(q->free_offset + entries_bytes, element_align);
    q->segment_bytes = q->slots_offset + (size_t)segment_len * element_size;
    q->block_align = element_align > block_align ? element_align : block_align;
    q->producers = 0u;
    q->consumers = 0u;
    first = sce_queue_lscq_new_segment(q);
    q->head = sce_queue_word_of(first);
    q->tail = sce_queue_word_of(first);
    return first != NULL;
}

/* Destroys the queue: gives every segment back, the ones consumers retired
 * included. The elements still in it are plain values and need nothing run. No
 * handle may be alive. */
static inline void sce_queue_lscq_destroy(sce_queue_lscq_t *q) {
    sce_queue_lscq_segment_t *segment = (sce_queue_lscq_segment_t *)sce_queue_pointer_of(q->head);
    while (segment != NULL) {
        sce_queue_lscq_segment_t *next =
            (sce_queue_lscq_segment_t *)sce_queue_pointer_of(sce_atomic_load_acquire_usize(&segment->next));
        sce_queue_lscq_free_segment(q, segment);
        segment = next;
    }
    q->head = 0u;
    q->tail = 0u;
    /* The segments consumers retired are on the domain's list and may still be
     * waiting for a scan; they must be freed before the allocator they name can go. */
    sce_hazard_flush(q->domain);
}

/* Takes a producer place and a slot of the domain: nonzero, with the slot's index
 * in `*hazard`, or zero when `places` producers are alive or the domain has no free
 * slot. */
static inline int sce_queue_lscq_producer_acquire(sce_queue_lscq_t *q, uint32_t *hazard) {
    int slot;
    if (!sce_queue_take_place(&q->producers, q->places)) {
        return 0;
    }
    slot = sce_hazard_acquire(q->domain);
    if (slot < 0) {
        (void)sce_atomic_fetch_sub_acq_rel_u32(&q->producers, 1u);
        return 0;
    }
    *hazard = (uint32_t)slot;
    return 1;
}

static inline void sce_queue_lscq_producer_release(sce_queue_lscq_t *q, uint32_t hazard) {
    sce_hazard_release(q->domain, hazard);
    (void)sce_atomic_fetch_sub_acq_rel_u32(&q->producers, 1u);
}

/* Takes a consumer place and a slot of the domain; see producer_acquire. */
static inline int sce_queue_lscq_consumer_acquire(sce_queue_lscq_t *q, uint32_t *hazard) {
    int slot;
    if (!sce_queue_take_place(&q->consumers, q->places)) {
        return 0;
    }
    slot = sce_hazard_acquire(q->domain);
    if (slot < 0) {
        (void)sce_atomic_fetch_sub_acq_rel_u32(&q->consumers, 1u);
        return 0;
    }
    *hazard = (uint32_t)slot;
    return 1;
}

static inline void sce_queue_lscq_consumer_release(sce_queue_lscq_t *q, uint32_t hazard) {
    sce_hazard_release(q->domain, hazard);
    (void)sce_atomic_fetch_sub_acq_rel_u32(&q->consumers, 1u);
}

/* Takes a copy of `*value` into the queue, or reports SCE_QUEUE_PUSH_OUT_OF_MEMORY
 * when the newest segment is closed and the allocator will not give another; the
 * value is the caller's either way. `hazard` is the slot the producer place came
 * with. Lock-free when the allocator is. */
static inline sce_queue_push_status_t sce_queue_lscq_try_push(sce_queue_lscq_t *q, uint32_t hazard, const void *value) {
    for (;;) {
        sce_queue_lscq_segment_t *tail =
            (sce_queue_lscq_segment_t *)sce_queue_pointer_of(sce_hazard_protect(q->domain, hazard, &q->tail));
        size_t next = sce_atomic_load_acquire_usize(&tail->next);
        sce_queue_lscq_segment_t *fresh;
        if (next != 0u) {
            /* A producer linked a successor and has not moved `tail`: finish its
             * step. */
            (void)sce_atomic_cas_strong_acq_rel_usize(&q->tail, sce_queue_word_of(tail), next);
            continue;
        }
        if (sce_queue_scq64_push_or_close(&tail->ring, sce_queue_lscq_slots(q, tail), q->element_size, value)) {
            sce_hazard_clear(q->domain, hazard);
            return SCE_QUEUE_PUSH_OK;
        }
        /* The segment is closed. Put the element in a segment of our own before
         * anyone can see it, then try to link it behind this one. */
        fresh = sce_queue_lscq_new_segment(q);
        if (fresh == NULL) {
            sce_hazard_clear(q->domain, hazard);
            return SCE_QUEUE_PUSH_OUT_OF_MEMORY;
        }
        /* A new ring is open and has a free slot, so it takes an element. */
        (void)sce_queue_scq64_push_or_close(&fresh->ring, sce_queue_lscq_slots(q, fresh), q->element_size, value);
        if (sce_atomic_cas_strong_acq_rel_usize(&tail->next, 0u, sce_queue_word_of(fresh)) == 0u) {
            (void)sce_atomic_cas_strong_acq_rel_usize(&q->tail, sce_queue_word_of(tail), sce_queue_word_of(fresh));
            sce_hazard_clear(q->domain, hazard);
            return SCE_QUEUE_PUSH_OK;
        }
        /* Another producer linked first. The element was copied, so the caller's
         * value is intact: free the segment nobody saw and push behind theirs. */
        sce_queue_lscq_free_segment(q, fresh);
    }
}

/* Takes the oldest element into `*out`, or returns zero when the queue is empty.
 * `hazard` is the slot the consumer place came with. Lock-free. */
static inline int sce_queue_lscq_try_pop(sce_queue_lscq_t *q, uint32_t hazard, void *out) {
    for (;;) {
        sce_queue_lscq_segment_t *head =
            (sce_queue_lscq_segment_t *)sce_queue_pointer_of(sce_hazard_protect(q->domain, hazard, &q->head));
        size_t head_word = sce_queue_word_of(head);
        size_t next;
        if (sce_queue_scq64_try_pop(&head->ring, sce_queue_lscq_slots(q, head), q->element_size, out)) {
            sce_hazard_clear(q->domain, hazard);
            return 1;
        }
        next = sce_atomic_load_acquire_usize(&head->next);
        if (next == 0u) {
            sce_hazard_clear(q->domain, hazard);
            return 0;
        }
        /* The segment is closed. An enqueue that took a ticket before the close may
         * still be filling its entry, so the empty answer above does not yet show
         * the segment holds nothing and never will. */
        if (sce_queue_scq64_pop_drained(&head->ring, sce_queue_lscq_slots(q, head), q->element_size, out)) {
            sce_hazard_clear(q->domain, hazard);
            return 1;
        }
        /* Take `tail` off the segment before it is unlinked from `head`: a producer
         * validates its hazard on the segment against `tail`, so `tail` must stop
         * naming the segment before the segment can be retired (the comment above,
         * *Where a hazard is validated*). The successor exists, so `tail` may
         * legally move to it; if another participant already moved it, the
         * exchange fails and reads that move. */
        (void)sce_atomic_cas_strong_acq_rel_usize(&q->tail, head_word, next);
        if (sce_atomic_cas_strong_acq_rel_usize(&q->head, head_word, next) == head_word) {
            /* This consumer moved `head` past the segment, so it alone retires it.
             * The fence puts the two compare-and-swaps before the scan the
             * retirement may start; the consumer's own hazard goes first, or that
             * scan would keep the segment for it. */
            sce_atomic_fence_seq_cst();
            sce_hazard_clear(q->domain, hazard);
            sce_hazard_retire(q->domain, &head->retired, sce_queue_lscq_reclaim, q);
        }
    }
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_QUEUE_SEGMENTED_H */
