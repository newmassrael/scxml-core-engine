/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the runtime half of `sce:kind="broadcast-log"` (ADR 0062) for C11:
 * one publisher, any number of readers, and the oldest entry forgotten when the
 * log is full. The same algorithm as the Rust runtime
 * (backends/rust/forge-runtime/src/broadcast_log.rs), with the same memory
 * orders and the same two fences, because the orders are part of the algorithm
 * and not a choice: a sequentially consistent model cannot tell a correct one
 * from a broken one, and the loom model of the Rust row found that a relaxed
 * busy stamp and a relaxed second stamp load are both faults.
 *
 * The contract is the sequential log of ADR 0062 section 2. `h` entries have
 * been published and only the last `retention` are retained. A reader owns a
 * cursor: sce_blog_read returns the entry at the cursor, or says the log has
 * nothing new, or says how many entries the reader missed and moves it to the
 * oldest retained. Publishing never fails and never waits, and neither does a
 * read: both are wait-free, in a number of steps that depends on the width of
 * an entry and on nothing the other side does.
 *
 * The protocol (ADR 0062 section 4). A slot holds a stamp and the entry's words.
 * The publisher writes a slot as `stamp := busy(n)` (release), a release fence,
 * the words (relaxed), `stamp := done(n)` (release), then `head := n + 1`
 * (release). A reader holding cursor `r` loads `head` (acquire) and answers
 * Empty at the head and Lagged outside the window; otherwise it loads the stamp
 * of slot `r mod slots` (acquire), copies the words (relaxed), executes an
 * acquire fence, and loads the stamp again (acquire): if it is still `done(r)`
 * the copy is entry `r`; if not, the publisher has reused the slot, and the
 * reader reloads `head` and answers Lagged. This is a sequence lock in the form
 * Boehm showed correct under the C++11 model (MSPC 2012): the words are atomic
 * accesses, so there is no data race, and the two fences carry the order.
 *
 * The slot count is `retention + 1`. The slot being overwritten is in flux and
 * cannot be promised to a reader; with `retention` slots a reader could find the
 * entry the specification retains already being overwritten and be told it
 * lagged by nothing (docs/formal/BroadcastLog.tla finds that state).
 *
 * The stamp is a 64-bit sequence number and not a flag, so a reader comparing
 * against `done(r)` cannot mistake a later lap for the entry it wants. It stays
 * below SCE_BLOG_HORIZON, 292 years at a billion publishes per second. A target
 * whose atomics do not reach 64 bits has no broadcast log.
 *
 * What differs from the languages with destructors and generics, as for the
 * queues (sce/forge/queue.h):
 *
 *   - The storage is the caller's: a control block and `slots * (1 + words)`
 *     64-bit words, laid out by the generated type for the document, so nothing
 *     here allocates. An entry is `words` 64-bit words, the wire form of the
 *     element (ADR 0049/0058) and never a value with ownership, so an entry has
 *     no destructor to run when it is overwritten.
 *   - A log is built by sce_blog_init and handed to the other participants after
 *     it returns. Nothing orders sce_blog_init against a concurrent user.
 *   - The one place of the publisher is taken with sce_blog_publisher_acquire and
 *     given back with sce_blog_publisher_release; its successor continues the
 *     sequence. A reader is a value of the caller's, so the log does not know how
 *     many exist.
 *   - Layout is not part of the contract: the Rust row gives `head` a cache line
 *     of its own, and a target that wants that lays the control block out so.
 *
 * The atomics are the `sce_atomic_*` externs of sce/forge/atomics.h, which the
 * platform implements; this header contains no compiler builtin and no
 * <stdatomic.h>.
 */

#ifndef SCE_FORGE_BROADCAST_LOG_H
#define SCE_FORGE_BROADCAST_LOG_H

#include <stddef.h>
#include <stdint.h>

#include <sce/forge/atomics.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The most entries a log can publish: the stamps are `2n + 2`, which must fit a
 * uint64_t. sce_blog_publish stops here and publishes nothing. */
#define SCE_BLOG_HORIZON ((UINT64_MAX - 2u) / 2u)

/* What sce_blog_publish returns when the log has published its horizon of
 * entries. It is not a sequence number: every sequence number is below the
 * horizon. */
#define SCE_BLOG_NO_SEQUENCE UINT64_MAX

/* The 64-bit words the caller's storage holds for a log of `slots` slots of
 * `words`-word entries. */
#define SCE_BLOG_STORAGE_WORDS(slots, words) ((size_t)(slots) * ((size_t)(words) + 1u))

/* What a read found. */
typedef enum sce_blog_read_status {
    /* The entry `seq` was copied into the caller's buffer, and the reader now
     * holds `seq + 1`. */
    SCE_BLOG_ITEM = 0,
    /* The reader is at the head: nothing newer is published. Its cursor is
     * unchanged. */
    SCE_BLOG_EMPTY = 1,
    /* The reader had fallen behind the window and missed `skipped >= 1` entries.
     * It now holds the oldest retained entry, so its next read returns that. The
     * caller's buffer is unspecified. */
    SCE_BLOG_LAGGED = 2
} sce_blog_read_status_t;

typedef struct sce_blog_read {
    sce_blog_read_status_t status;
    /* The sequence number of the entry copied; meaningful for SCE_BLOG_ITEM. */
    uint64_t seq;
    /* How many entries the reader missed; meaningful for SCE_BLOG_LAGGED. */
    uint64_t skipped;
} sce_blog_read_t;

/* A reader's position, owned by the reader and shared with nothing. Opening one
 * costs the log nothing: the publisher does not know how many readers exist,
 * which is what keeps publishing wait-free. */
typedef struct sce_blog_reader {
    /* The sequence number this reader reads next. */
    uint64_t cursor;
    /* The entries this reader has been told it missed, summed over its reads. */
    uint64_t lost;
} sce_blog_reader_t;

/* A bounded log whose entries are `words` 64-bit words, kept in `slots` slots and
 * retaining the last `slots - 1` entries. Each slot is its stamp followed by its
 * words, in `storage`. */
typedef struct sce_blog {
    /* The number of entries published. Written only by the publisher, loaded by
     * every reader. */
    uint64_t head;
    /* Whether a publisher exists. */
    uint32_t publisher_taken;
    uint32_t slots;
    uint32_t words;
    uint64_t *storage;
} sce_blog_t;

/* The publishing side of a log. There is one at a time. */
typedef struct sce_blog_publisher {
    sce_blog_t *log;
    /* The sequence number of the next entry: this side's own count, so publishing
     * reads no shared state. */
    uint64_t next;
} sce_blog_publisher_t;

/* The stamp a slot holds while the entry `n` is being written into it. */
static inline uint64_t sce_blog_busy(uint64_t n) {
    return 2u * n + 1u;
}

/* The stamp a slot holds once the entry `n` is in it. */
static inline uint64_t sce_blog_done(uint64_t n) {
    return 2u * n + 2u;
}

/* Builds an empty log of `slots` slots of `words`-word entries over `storage`,
 * which holds SCE_BLOG_STORAGE_WORDS(slots, words) words and is written (to
 * zero) here. A log retains at least one entry and an entry holds at least one
 * word: returns zero, and builds nothing, for `slots` below two, `words` of zero
 * or a null `storage`. */
static inline int sce_blog_init(sce_blog_t *log, uint64_t *storage, uint32_t slots, uint32_t words) {
    size_t i;
    const size_t total = SCE_BLOG_STORAGE_WORDS(slots, words);
    if (storage == NULL || slots < 2u || words == 0u) {
        return 0;
    }
    for (i = 0; i < total; i++) {
        storage[i] = 0;
    }
    log->head = 0;
    log->publisher_taken = 0;
    log->slots = slots;
    log->words = words;
    log->storage = storage;
    return 1;
}

/* The entries a reader may rely on. */
static inline uint64_t sce_blog_retention(const sce_blog_t *log) {
    return (uint64_t)log->slots - 1u;
}

/* The number of entries published so far. */
static inline uint64_t sce_blog_head(const sce_blog_t *log) {
    return sce_atomic_load_acquire_u64(&log->head);
}

/* The first word of the slot that holds sequence number `seq`: its stamp, then
 * its words. */
static inline uint64_t *sce_blog_slot(const sce_blog_t *log, uint64_t seq) {
    return log->storage + (size_t)(seq % (uint64_t)log->slots) * ((size_t)log->words + 1u);
}

/* Takes the publisher's place, or returns zero while another is held: a log
 * written by two publishers is not this algorithm. The publisher continues the
 * sequence where the last one stopped. */
static inline int sce_blog_publisher_acquire(sce_blog_t *log, sce_blog_publisher_t *publisher) {
    if (sce_atomic_cas_strong_acq_rel_u32(&log->publisher_taken, 0u, 1u) != 0u) {
        return 0;
    }
    publisher->log = log;
    publisher->next = sce_atomic_load_acquire_u64(&log->head);
    return 1;
}

/* Gives the publisher's place back. */
static inline void sce_blog_publisher_release(sce_blog_publisher_t *publisher) {
    sce_atomic_store_release_u32(&publisher->log->publisher_taken, 0u);
}

/* Publishes `entry` (`words` 64-bit words) and returns the sequence number it
 * took. Never fails and never waits, except that a log which has published its
 * horizon of entries publishes nothing more and returns SCE_BLOG_NO_SEQUENCE. */
static inline uint64_t sce_blog_publish(sce_blog_publisher_t *publisher, const uint64_t *entry) {
    sce_blog_t *log = publisher->log;
    const uint64_t n = publisher->next;
    uint64_t *slot;
    uint32_t k;
    if (n >= SCE_BLOG_HORIZON) {
        return SCE_BLOG_NO_SEQUENCE;
    }
    slot = sce_blog_slot(log, n);
    /* Release, for the reader that sees this stamp and then reloads `head`: it
     * must find `head` at `n` or later, and the store of `head` to `n` ended the
     * previous publish, in this thread's order before this one. A relaxed store
     * here lets that reload return an older `head`, and the reader then reports a
     * lag that skipped nothing. The fence below is still needed: a release store
     * orders what came before it, and this stamp must also come before the
     * words. */
    sce_atomic_store_release_u64(&slot[0], sce_blog_busy(n));
    sce_atomic_fence_release();
    for (k = 0; k < log->words; k++) {
        sce_atomic_store_relaxed_u64(&slot[1u + k], entry[k]);
    }
    sce_atomic_store_release_u64(&slot[0], sce_blog_done(n));
    /* The linearization point: a reader that loads this index (acquire) sees the
     * slot's stamp and words. */
    sce_atomic_store_release_u64(&log->head, n + 1u);
    publisher->next = n + 1u;
    return n;
}

/* A reader at the oldest retained entry. */
static inline sce_blog_reader_t sce_blog_reader_oldest(const sce_blog_t *log) {
    sce_blog_reader_t reader;
    const uint64_t head = sce_blog_head(log);
    const uint64_t retention = sce_blog_retention(log);
    reader.cursor = head > retention ? head - retention : 0u;
    reader.lost = 0;
    return reader;
}

/* A reader at the head: it sees only what is published after this call. */
static inline sce_blog_reader_t sce_blog_reader_head(const sce_blog_t *log) {
    sce_blog_reader_t reader;
    reader.cursor = sce_blog_head(log);
    reader.lost = 0;
    return reader;
}

/* A reader at the sequence number `seq`, or at the head when `seq` is past it. */
static inline sce_blog_reader_t sce_blog_reader_at(const sce_blog_t *log, uint64_t seq) {
    sce_blog_reader_t reader;
    const uint64_t head = sce_blog_head(log);
    reader.cursor = seq < head ? seq : head;
    reader.lost = 0;
    return reader;
}

/* Moves `reader` to the oldest entry retained when the head is `head`, and says
 * how far that was. */
static inline sce_blog_read_t sce_blog_lagged(sce_blog_reader_t *reader, const sce_blog_t *log, uint64_t head) {
    sce_blog_read_t result;
    const uint64_t retention = sce_blog_retention(log);
    const uint64_t oldest = head > retention ? head - retention : 0u;
    const uint64_t skipped = oldest > reader->cursor ? oldest - reader->cursor : 0u;
    reader->cursor = oldest;
    reader->lost = skipped > UINT64_MAX - reader->lost ? UINT64_MAX : reader->lost + skipped;
    result.status = SCE_BLOG_LAGGED;
    result.seq = 0;
    result.skipped = skipped;
    return result;
}

/* Reads the entry at `reader`'s cursor into `out` (`words` 64-bit words). A
 * bounded number of steps whatever the publisher is doing: a copy that the
 * publisher overwrote is not retried, it is the answer (SCE_BLOG_LAGGED). */
static inline sce_blog_read_t sce_blog_read(const sce_blog_t *log, sce_blog_reader_t *reader, uint64_t *out) {
    sce_blog_read_t result;
    const uint64_t r = reader->cursor;
    uint64_t head = sce_atomic_load_acquire_u64(&log->head);
    result.seq = 0;
    result.skipped = 0;
    if (r >= head) {
        result.status = SCE_BLOG_EMPTY;
        return result;
    }
    if (head - r >= (uint64_t)log->slots) {
        return sce_blog_lagged(reader, log, head);
    }
    {
        /* `r` was in the window when `head` was loaded, so its slot was filled
         * before that load. It holds `r` still unless the publisher has since
         * begun writing `r + slots`. */
        const uint64_t *slot = sce_blog_slot(log, r);
        const uint64_t first = sce_atomic_load_acquire_u64(&slot[0]);
        if (first == sce_blog_done(r)) {
            uint32_t k;
            for (k = 0; k < log->words; k++) {
                out[k] = sce_atomic_load_relaxed_u64(&slot[1u + k]);
            }
            /* The fence keeps the copy before the second load, which is what makes
             * an unchanged stamp mean an unchanged copy. The load is an acquire for
             * the other reason: if it sees the stamp of a later lap, the reload of
             * `head` below must see the publish that began that lap. A fence orders
             * a later load only against reads before it, so this load cannot rely
             * on the fence for that, and a relaxed one reports a lag that skipped
             * nothing. */
            sce_atomic_fence_acquire();
            if (sce_atomic_load_acquire_u64(&slot[0]) == first) {
                reader->cursor = r + 1u;
                result.status = SCE_BLOG_ITEM;
                result.seq = r;
                return result;
            }
        }
    }
    /* The slot was reused while it was being read, so the publisher has published
     * through `r + retention` and `head` shows it. */
    head = sce_atomic_load_acquire_u64(&log->head);
    return sce_blog_lagged(reader, log, head);
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_BROADCAST_LOG_H */
