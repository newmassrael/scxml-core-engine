// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inbox of a generated C11 machine: the one part of it that a thread, a
// core or an interrupt handler that does not own the machine may use
// (docs/adr/0084, ARCHITECTURE.md "Engine Threading").
//
// A multi-producer, single-consumer queue of what other contexts have for the
// machine, which its owner applies at its next macrostep boundary. What an
// input is belongs to the machine (`<machine>_inbox_input_t`, a closed set the
// generated header declares); this header holds the queue, which is the same
// for every machine, and the machine holds the storage of its slots, so nothing
// allocates.
//
// Bounded, with the capacity the deployment declares (ADR 0084, D2):
// `-DSCE_MAX_INBOX=N`, the same build-time override every other bound of this
// runtime takes. A deployment that declares none has no inbox: the machine
// carries no queue and declares no post, so a superloop that hears from nobody
// pays nothing, links no atomics, and a host that posts to it does not compile.
// A full inbox refuses a post and says so: the post returns false and the
// refusal is counted. Nothing is dropped silently.
//
// The queue is the forge runtime's (sce/forge/queue.h), on the row the target's
// atomics allow. The deployment states which, as the forge deploy's
// `platform.atomic_rmw_width` does (`-DSCE_INBOX_ATOMIC_RMW_WIDTH=W`):
//
//   64 (the default)  Nikolaev's SCQ over 64-bit entries, lock-free on both
//                     sides. A post never waits for the context it interrupted,
//                     so an interrupt handler posts through it as a thread does.
//   32                the same SCQ over 32-bit entries, for a target whose
//                     atomics reach 32 bits only.
//   0                 one ring under an interrupt-masked critical section
//                     (sce/forge/queue_irq.h), for a single-core target with no
//                     read-modify-write atomic: every post and every take masks
//                     interrupts for the length of one copy.
//
// The platform supplies what the row calls: the `sce_atomic_*` symbols
// (sce/forge/atomics.h) for 64 and 32, `sce_irq_save` and `sce_irq_restore`
// for 0. An SCQ row may refuse a post while other posts, or the owner's take,
// hold slots (at most one each), as the queue it is built on says; that is a
// refusal for room like any other.

#ifndef SCE_INBOX_H
#define SCE_INBOX_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/types.h"

/* docs/adr/0084: the most inputs a machine's inbox holds, a power of two on an
   SCQ row; 0, the default, for no inbox. Declared by the deployment, because only
   the deployment knows how many contexts post to its machines and how far its
   owner may fall behind them. */
#ifndef SCE_MAX_INBOX
#define SCE_MAX_INBOX 0
#endif

#if SCE_MAX_INBOX > 0

#ifndef SCE_INBOX_ATOMIC_RMW_WIDTH
#define SCE_INBOX_ATOMIC_RMW_WIDTH 64
#endif

#include "sce/portability.h"
#include <sce/forge/queue.h>

#if SCE_INBOX_ATOMIC_RMW_WIDTH == 0
#include <sce/forge/queue_irq.h>
#elif SCE_INBOX_ATOMIC_RMW_WIDTH != 64 && SCE_INBOX_ATOMIC_RMW_WIDTH != 32
#error "SCE_INBOX_ATOMIC_RMW_WIDTH is the target's widest read-modify-write atomic: 64, 32 or 0"
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* The row's names, so each operation below is written once. */
#if SCE_INBOX_ATOMIC_RMW_WIDTH == 64
#define SCE_INBOX_ROW_(name) sce_queue_scq64_##name
typedef uint64_t sce_inbox_entry_t;
#elif SCE_INBOX_ATOMIC_RMW_WIDTH == 32
#define SCE_INBOX_ROW_(name) sce_queue_scq32_##name
typedef uint32_t sce_inbox_entry_t;
#else
#define SCE_INBOX_ROW_(name) sce_queue_irq_##name
#endif

#if SCE_INBOX_ATOMIC_RMW_WIDTH != 0
/* An SCQ ring has a power of two of slots, and the inbox's rings have exactly
   its capacity, so the capacity is the ring and holds exactly what it says. */
SCE_STATIC_ASSERT((SCE_MAX_INBOX & (SCE_MAX_INBOX - 1)) == 0,
                  "SCE_MAX_INBOX must be a power of two on an SCQ row (SCE_INBOX_ATOMIC_RMW_WIDTH 64 or 32)");
#if SCE_INBOX_ATOMIC_RMW_WIDTH == 64
SCE_STATIC_ASSERT(SCE_MAX_INBOX <= (1L << 30), "SCE_MAX_INBOX is larger than a 64-bit SCQ entry can number");
#else
SCE_STATIC_ASSERT(SCE_MAX_INBOX <= (1L << 15), "SCE_MAX_INBOX is larger than a 32-bit SCQ entry can number");
#endif
#endif

/* What a post calls, on the posting context, after the inbox took the input:
   how a host that drives the machine itself (`_step`, `_tick`) learns there is
   something to apply. On an interrupt handler's post it runs in that handler. */
typedef void (*sce_inbox_waker_fn)(void *context);

/* The queue, the entries of its index rings, the count of refused posts and the
   waker. The slots the inputs are copied into are the machine's own array of its
   input type, passed to each call with the size of one. */
typedef struct sce_inbox_s {
    SCE_INBOX_ROW_(t) queue;
#if SCE_INBOX_ATOMIC_RMW_WIDTH != 0
    sce_inbox_entry_t allocated_entries[2u * SCE_MAX_INBOX];
    sce_inbox_entry_t free_entries[2u * SCE_MAX_INBOX];
#endif
    uint32_t refused;
    sce_inbox_waker_fn waker;
    void *waker_context;
} sce_inbox_t;

/* Build an empty inbox with no waker. The owner's, before the machine is handed
   to another context: nothing orders it against a concurrent post. False only
   for a shape the queue refuses, which the assertions above rule out. */
static inline bool sce_inbox_init(sce_inbox_t *inbox) {
    inbox->refused = 0u;
    inbox->waker = NULL;
    inbox->waker_context = NULL;
#if SCE_INBOX_ATOMIC_RMW_WIDTH == 0
    return sce_queue_irq_init(&inbox->queue, (uint32_t)SCE_MAX_INBOX, (uint32_t)SCE_MAX_INBOX, 1u) != 0;
#else
    return SCE_INBOX_ROW_(init)(&inbox->queue, (uint32_t)SCE_MAX_INBOX, (uint32_t)SCE_MAX_INBOX,
                                inbox->allocated_entries, inbox->free_entries) != 0;
#endif
}

/* Install `waker`, called with `context` after each post the inbox takes; NULL
   removes it. The owner's, before the machine is handed to another context, for
   the reason `sce_inbox_init` is: a post reads the pair without a lock. */
static inline void sce_inbox_wake_with(sce_inbox_t *inbox, sce_inbox_waker_fn waker, void *context) {
    inbox->waker = waker;
    inbox->waker_context = context;
}

/* Count one refused post. Any context's. */
static inline void sce_inbox_count_refusal(sce_inbox_t *inbox) {
#if SCE_INBOX_ATOMIC_RMW_WIDTH == 0
    const irq_state_t state = sce_irq_save();
    inbox->refused++;
    sce_irq_restore(state);
#else
    (void)sce_atomic_fetch_add_acq_rel_u32(&inbox->refused, 1u);
#endif
}

/* How many posts the inbox refused since it was built. Any context's. */
static inline uint32_t sce_inbox_refused(const sce_inbox_t *inbox) {
#if SCE_INBOX_ATOMIC_RMW_WIDTH == 0
    const irq_state_t state = sce_irq_save();
    const uint32_t refused = inbox->refused;
    sce_irq_restore(state);
    return refused;
#else
    return sce_atomic_load_acquire_u32(&inbox->refused);
#endif
}

/* Copy `element_size` bytes at `input` into the inbox whose slots are `slots`;
   whether the inbox took them. Any context's. A refusal is counted; after a post
   the inbox took, the waker is called on this context. */
static inline bool sce_inbox_post(sce_inbox_t *inbox, unsigned char *slots, size_t element_size, const void *input) {
    bool taken = false;
    if (SCE_INBOX_ROW_(producer_acquire)(&inbox->queue)) {
        taken = SCE_INBOX_ROW_(try_push)(&inbox->queue, slots, element_size, input) == SCE_QUEUE_PUSH_OK;
        SCE_INBOX_ROW_(producer_release)(&inbox->queue);
    }
    if (!taken) {
        sce_inbox_count_refusal(inbox);
        return false;
    }
    if (inbox->waker != NULL) {
        inbox->waker(inbox->waker_context);
    }
    return true;
}

/* Take the oldest input into `out`; false when the inbox holds none the owner
   can reach yet. The owner's. */
static inline bool sce_inbox_take(sce_inbox_t *inbox, unsigned char *slots, size_t element_size, void *out) {
    bool taken = false;
    if (SCE_INBOX_ROW_(consumer_acquire)(&inbox->queue)) {
        taken = SCE_INBOX_ROW_(try_pop)(&inbox->queue, slots, element_size, out) != 0;
        SCE_INBOX_ROW_(consumer_release)(&inbox->queue);
    }
    return taken;
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_MAX_INBOX > 0 */

#endif /* SCE_INBOX_H */
