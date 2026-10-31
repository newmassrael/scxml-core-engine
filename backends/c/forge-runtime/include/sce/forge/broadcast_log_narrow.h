/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge -- the narrow row of the runtime half of `sce:kind="broadcast-log"`
 * (ADR 0062, "Stamp width") for C11: the log of sce/forge/broadcast_log.h for a
 * target whose atomics are 32 bits wide or only loads and stores (Cortex-M0 and
 * M3, RISC-V 32). It is the C counterpart of the Rust runtime's narrow row
 * (backends/rust/forge-runtime/src/broadcast_log_core.rs, the lane `Narrow<B>`,
 * and broadcast_log_narrow.rs), with the same memory orders, the same two fences
 * and the same arithmetic, and it is a separate header and not the 64-bit one
 * made generic so that the 64-bit row's text and its generated headers stay as
 * they were.
 *
 * The stamp, the head and every word of an entry are `uint32_t` objects accessed
 * through the u32 `sce_atomic_*` family of sce/forge/atomics.h: acquire and
 * relaxed loads, release and relaxed stores, and the two fences. The row needs no
 * 64-bit atomic and no read-modify-write instruction; the one function that does,
 * sce_blogn_publisher_acquire (a 32-bit compare-and-swap), has an ownership form
 * beside it for a target that has none (see "The publisher" below), and a header
 * that never calls it never references the symbol.
 *
 * The protocol is the 64-bit row's (sce/forge/broadcast_log.h): the publisher
 * writes a slot as `stamp := busy(n)` (release), a release fence, the words
 * (relaxed), `stamp := done(n)` (release), then `head := n + 1` (release); a reader
 * holding cursor `r` loads `head` (acquire), answers Empty at the head and Lagged
 * outside the window, and otherwise loads the stamp of slot `r mod slots`
 * (acquire), copies the words (relaxed), executes an acquire fence and loads the
 * stamp again (acquire): if it is still `done(r)` the copy is entry `r`; if not,
 * the publisher reused the slot, and the reader reloads `head` and answers Lagged.
 * The slot count is `retention + 1`.
 *
 * What a narrow stamp changes is arithmetic, written once in terms of the stamp
 * space `M = 2^w`, `w` = SCE_BLOGN_STAMP_BITS:
 *
 *   - The stored stamps and head are the low `w` bits of the count they stand for:
 *     busy(n) = (2n + 1) mod M and done(n) = (2n + 2) mod M, so a stamp tells apart
 *     K = M / 2 consecutive entries of one slot, and the head stores (n + 1) mod M.
 *   - A sequence number is a count modulo M: sce_blogn_head, a reader's cursor, the
 *     `seq` of an item and the number a publish returns are residues, the same for
 *     every handle, and they are not ordered across a wrap. A reader takes its
 *     distance from the head as (head - cursor) mod M, which is the true distance
 *     while it is below M.
 *   - The slot count divides M, so that the slot index `n mod slots`, which a
 *     reader knows only from a residue, is the publisher's: a power of two, at most
 *     K. sce_blogn_init refuses another shape.
 *   - The wrap bound. A reader gets the answers of the specification of ADR 0062
 *     section 2 while, during each read, the newest publish begun is at most K past
 *     its cursor. A deploy states it as the longest pause in publishes between two
 *     of a reader's reads, `min_wrap_ops`, which must not exceed K - slots; the
 *     generator refuses a log whose bound does not cover it. A reader that breaks
 *     the bound is not told: what it is answered is defined (below), not undefined.
 *
 * Outside the bound the answer is the arithmetic of the cursor modulo M. The
 * cursor is always right modulo M (Empty leaves it at the head's residue, Lagged at
 * the oldest retained entry's); `skipped` is the distance taken modulo M less the
 * retention, so it is short by a multiple of M after that many laps, and a reader
 * exactly M behind is told Empty; and a copy that validated while the reader was K
 * or more behind is the entry a later lap wrote, handed as the one wanted (the
 * stamp is the same value one lap on).
 *
 * UNWRITTEN. A count modulo M cannot tell a log that has published fewer than
 * `retention` entries from one that has published more and wrapped, which
 * sce_blogn_reader_oldest needs to know. It asks the last slot: that slot is first
 * written by the publish of entry `retention`, so a stamp of SCE_BLOGN_UNWRITTEN
 * there means at most `retention` entries were published and the oldest retained
 * is entry 0. The value is 1, which no write into the last slot produces (the last
 * slot holds the entries n = slots - 1 modulo slots, stamped 2 * slots - 1 and 0
 * modulo 2 * slots, and 2 * slots divides M); sce_blogn_init stamps every slot with
 * it.
 *
 * The publisher. There is one at a time, and the algorithm needs no
 * read-modify-write to be one: the single writer is an assumption about who calls
 * sce_blogn_publish. Three ways to take the place, in the order of how much each
 * asks of the target and of the caller:
 *
 *   - sce_blogn_publisher_acquire takes it with a 32-bit compare-and-swap and
 *     returns zero while another is held. A target without one (a core with loads
 *     and stores only) does not implement `sce_atomic_cas_strong_acq_rel_u32` and
 *     does not call this.
 *   - sce_blogn_split takes it from a log the caller holds exclusively, before it
 *     is shared (what the exclusive borrow of Rust's `split` proves): the check is
 *     a load and a store, so it is exact only when no other thread can be taking
 *     the place at the same time, and it refuses a place that is held.
 *   - sce_blogn_publisher_unchecked takes it on the caller's word, for a log in
 *     static storage that is shared from the start.
 *
 * All three are one place: each raises the flag the others look at, and
 * sce_blogn_publisher_release gives it back; a successor continues the sequence
 * from the head.
 *
 * What is the 64-bit row's, as for the queues (sce/forge/queue.h): the storage is
 * the caller's, a control block and `slots * (1 + words)` 32-bit words laid out by
 * the generated type, so nothing here allocates; an entry is `words` 32-bit words,
 * the wire form of the element (ADR 0049/0058) and never a value with ownership; a
 * log is built by sce_blogn_init and handed to the other participants after it
 * returns; a reader is a value of the caller's, so the log does not know how many
 * exist.
 *
 * The stamp width is a compile-time parameter, SCE_BLOGN_STAMP_BITS, 32 unless the
 * translation unit defines it before this header: a unit test uses 2 or 3 bits so
 * that the wrap is a few publishes away, as the Rust lane's parameter does. A row
 * keeps 32, and the generated header asserts it.
 *
 * The atomics are the `sce_atomic_*` externs of sce/forge/atomics.h, which the
 * platform implements; this header contains no compiler builtin and no
 * <stdatomic.h>.
 */

#ifndef SCE_FORGE_BROADCAST_LOG_NARROW_H
#define SCE_FORGE_BROADCAST_LOG_NARROW_H

#include <stddef.h>
#include <stdint.h>

#include <sce/forge/atomics.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The stamp's width in bits, `w`: the log counts modulo `2^w`. */
#ifndef SCE_BLOGN_STAMP_BITS
#define SCE_BLOGN_STAMP_BITS 32u
#endif

/* The words are 32-bit, so a stamp is at most 32 bits; it needs a bit for busy
 * against done and one for the lap. A width outside that makes this array's size
 * negative, which the compiler refuses. */
typedef char
    sce_blogn_stamp_bits_between_2_and_32[(SCE_BLOGN_STAMP_BITS >= 2u && SCE_BLOGN_STAMP_BITS <= 32u) ? 1 : -1];

/* The low `w` bits. */
#define SCE_BLOGN_MASK ((uint32_t)(UINT32_MAX >> (32u - SCE_BLOGN_STAMP_BITS)))

/* The number of values a stamp takes, `2^w`. */
#define SCE_BLOGN_SPACE ((uint64_t)SCE_BLOGN_MASK + 1u)

/* Half the stamp space, `K`: how many consecutive entries of one slot a stamp
 * tells apart, and the most slots a log keeps. */
#define SCE_BLOGN_HALF ((uint32_t)(UINT32_C(1) << (SCE_BLOGN_STAMP_BITS - 1u)))

/* The stamp of a slot nothing has been written to (see "UNWRITTEN" above). */
#define SCE_BLOGN_UNWRITTEN UINT32_C(1)

/* The 32-bit words the caller's storage holds for a log of `slots` slots of
 * `words`-word entries. */
#define SCE_BLOGN_STORAGE_WORDS(slots, words) ((size_t)(slots) * ((size_t)(words) + 1u))

/* What a read found. */
typedef enum sce_blogn_read_status {
    /* The entry `seq` was copied into the caller's buffer, and the reader now
     * holds `seq + 1` (modulo 2^w). */
    SCE_BLOGN_ITEM = 0,
    /* The reader is at the head: nothing newer is published. Its cursor is
     * unchanged. */
    SCE_BLOGN_EMPTY = 1,
    /* The reader had fallen behind the window and missed `skipped >= 1` entries.
     * It now holds the oldest retained entry, so its next read returns that. The
     * caller's buffer is unspecified. */
    SCE_BLOGN_LAGGED = 2
} sce_blogn_read_status_t;

typedef struct sce_blogn_read {
    sce_blogn_read_status_t status;
    /* The sequence number of the entry copied, modulo 2^w; meaningful for
     * SCE_BLOGN_ITEM. */
    uint32_t seq;
    /* How many entries the reader missed; meaningful for SCE_BLOGN_LAGGED. 64 bits,
     * because a lag from a head that wrapped is counted one lap on, and a lap is
     * 2^32 entries for a stamp of 32 bits. */
    uint64_t skipped;
} sce_blogn_read_t;

/* A reader's position, owned by the reader and shared with nothing. Opening one
 * costs the log nothing: the publisher does not know how many readers exist,
 * which is what keeps publishing wait-free. */
typedef struct sce_blogn_reader {
    /* The sequence number this reader reads next, modulo 2^w. A deferred reader
     * that has not read yet has none: this is 0 and sce_blogn_reader_is_bound says
     * no. */
    uint32_t cursor;
    /* The entries this reader has been told it missed, summed over its reads. */
    uint64_t lost;
    /* Nonzero for a reader opened by sce_blogn_reader_deferred, until its first
     * read binds a cursor. Every other constructor leaves it zero. */
    uint32_t unbound;
} sce_blogn_reader_t;

/* A bounded log whose entries are `words` 32-bit words, kept in `slots` slots (a
 * power of two, at most SCE_BLOGN_HALF) and retaining the last `slots - 1`. Each
 * slot is its stamp followed by its words, in `storage`. */
typedef struct sce_blogn {
    /* The number of entries published, modulo 2^w. Written only by the publisher,
     * loaded by every reader. */
    uint32_t head;
    /* Whether a publisher exists. */
    uint32_t publisher_taken;
    uint32_t slots;
    uint32_t words;
    uint32_t *storage;
} sce_blogn_t;

/* The publishing side of a log. There is one at a time. */
typedef struct sce_blogn_publisher {
    sce_blogn_t *log;
    /* The sequence number of the next entry, modulo 2^w: this side's own count, so
     * publishing reads no shared state. */
    uint32_t next;
} sce_blogn_publisher_t;

/* The stamp a slot holds while the entry `n` is being written into it. */
static inline uint32_t sce_blogn_busy(uint32_t n) {
    return (uint32_t)((n * UINT32_C(2) + UINT32_C(1)) & SCE_BLOGN_MASK);
}

/* The stamp a slot holds once the entry `n` is in it. */
static inline uint32_t sce_blogn_done(uint32_t n) {
    return (uint32_t)((n * UINT32_C(2) + UINT32_C(2)) & SCE_BLOGN_MASK);
}

/* How far `cursor` is behind `head`, both counts modulo 2^w: the true distance
 * whenever that is below 2^w. */
static inline uint32_t sce_blogn_distance(uint32_t head, uint32_t cursor) {
    return (uint32_t)((head - cursor) & SCE_BLOGN_MASK);
}

/* `cursor` moved `by` entries on, modulo 2^w. */
static inline uint32_t sce_blogn_advance(uint32_t cursor, uint64_t by) {
    return (uint32_t)(((uint64_t)cursor + by) & (uint64_t)SCE_BLOGN_MASK);
}

/* Builds an empty log of `slots` slots of `words`-word entries over `storage`,
 * which holds SCE_BLOGN_STORAGE_WORDS(slots, words) words and is written here: the
 * words to zero and every stamp to SCE_BLOGN_UNWRITTEN. A log retains at least one
 * entry, an entry holds at least one word, and the slot count divides the stamp
 * space: returns zero, and builds nothing, for a null `storage`, `slots` below
 * two, `slots` that is not a power of two or is above SCE_BLOGN_HALF, or `words`
 * of zero. */
static inline int sce_blogn_init(sce_blogn_t *log, uint32_t *storage, uint32_t slots, uint32_t words) {
    size_t i;
    uint32_t s;
    const size_t total = SCE_BLOGN_STORAGE_WORDS(slots, words);
    if (storage == NULL || slots < 2u || words == 0u) {
        return 0;
    }
    if ((slots & (slots - 1u)) != 0u || (uint64_t)slots > (uint64_t)SCE_BLOGN_HALF) {
        return 0;
    }
    for (i = 0; i < total; i++) {
        storage[i] = 0u;
    }
    for (s = 0; s < slots; s++) {
        storage[(size_t)s * ((size_t)words + 1u)] = SCE_BLOGN_UNWRITTEN;
    }
    log->head = 0u;
    log->publisher_taken = 0u;
    log->slots = slots;
    log->words = words;
    log->storage = storage;
    return 1;
}

/* The entries a reader may rely on. */
static inline uint32_t sce_blogn_retention(const sce_blogn_t *log) {
    return log->slots - 1u;
}

/* The number of entries published so far, modulo 2^w. */
static inline uint32_t sce_blogn_head(const sce_blogn_t *log) {
    return sce_atomic_load_acquire_u32(&log->head);
}

/* The first word of the slot that holds sequence number `seq` (a residue): its
 * stamp, then its words. The slot count divides the stamp space, so the index is
 * the publisher's whatever the lap. */
static inline uint32_t *sce_blogn_slot(const sce_blogn_t *log, uint32_t seq) {
    return log->storage + (size_t)(seq % log->slots) * ((size_t)log->words + 1u);
}

/* Takes the publisher's place with a 32-bit compare-and-swap, or returns zero
 * while another is held: a log written by two publishers is not this algorithm.
 * The publisher continues the sequence where the last one stopped. Needs the
 * platform's `sce_atomic_cas_strong_acq_rel_u32`; a target without a
 * read-modify-write uses sce_blogn_split or sce_blogn_publisher_unchecked. */
static inline int sce_blogn_publisher_acquire(sce_blogn_t *log, sce_blogn_publisher_t *publisher) {
    if (sce_atomic_cas_strong_acq_rel_u32(&log->publisher_taken, 0u, 1u) != 0u) {
        return 0;
    }
    publisher->log = log;
    publisher->next = sce_atomic_load_acquire_u32(&log->head);
    return 1;
}

/* Takes the publisher's place from a log the caller holds exclusively, with a load
 * and a store and no read-modify-write, or returns zero while a place is held. The
 * caller guarantees that no other thread is taking the place, or reading
 * `publisher_taken`, at this moment: the log is not yet shared, or every other
 * participant is known to be stopped. The place is then one with
 * sce_blogn_publisher_acquire's, and the publisher continues the sequence. */
static inline int sce_blogn_split(sce_blogn_t *log, sce_blogn_publisher_t *publisher) {
    if (sce_atomic_load_acquire_u32(&log->publisher_taken) != 0u) {
        return 0;
    }
    sce_atomic_store_release_u32(&log->publisher_taken, 1u);
    publisher->log = log;
    publisher->next = sce_atomic_load_acquire_u32(&log->head);
    return 1;
}

/* The publisher of a log that is shared from the start (a log in static storage),
 * taken on the caller's word. The caller guarantees, for as long as the publisher
 * is used:
 *   - no other publisher of this log exists, whether from this function, from
 *     sce_blogn_split or from sce_blogn_publisher_acquire: two publishers write the
 *     same slots and break the stamp protocol silently, with readers told entries
 *     that were never published or lags that skipped nothing;
 *   - the last publish of any earlier publisher of this log happens before this
 *     call (program order, a thread join, or a release store the caller acquires),
 *     so that the sequence this publisher continues from `head` is the one that
 *     was published.
 * The place is marked taken, so a later sce_blogn_publisher_acquire refuses. */
static inline sce_blogn_publisher_t sce_blogn_publisher_unchecked(sce_blogn_t *log) {
    sce_blogn_publisher_t publisher;
    sce_atomic_store_release_u32(&log->publisher_taken, 1u);
    publisher.log = log;
    publisher.next = sce_atomic_load_acquire_u32(&log->head);
    return publisher;
}

/* Gives the publisher's place back. */
static inline void sce_blogn_publisher_release(sce_blogn_publisher_t *publisher) {
    sce_atomic_store_release_u32(&publisher->log->publisher_taken, 0u);
}

/* Publishes `entry` (`words` 32-bit words) and returns the sequence number it
 * took, modulo 2^w. Never fails and never waits, and has no horizon: the count
 * wraps, and every value of it is a sequence number. */
static inline uint32_t sce_blogn_publish(sce_blogn_publisher_t *publisher, const uint32_t *entry) {
    sce_blogn_t *log = publisher->log;
    const uint32_t n = publisher->next;
    uint32_t *const slot = sce_blogn_slot(log, n);
    uint32_t next;
    uint32_t k;
    /* Release, for the reader that sees this stamp and then reloads `head`: it
     * must find `head` at `n` or later, and the store of `head` to `n` ended the
     * previous publish, in this thread's order before this one. A relaxed store
     * here lets that reload return an older `head`, and the reader then reports a
     * lag that skipped nothing. The fence below is still needed: a release store
     * orders what came before it, and this stamp must also come before the
     * words. */
    sce_atomic_store_release_u32(&slot[0], sce_blogn_busy(n));
    sce_atomic_fence_release();
    for (k = 0; k < log->words; k++) {
        sce_atomic_store_relaxed_u32(&slot[1u + k], entry[k]);
    }
    sce_atomic_store_release_u32(&slot[0], sce_blogn_done(n));
    /* The linearization point: a reader that loads this index (acquire) sees the
     * slot's stamp and words. */
    next = (uint32_t)sce_blogn_advance(n, 1u);
    sce_atomic_store_release_u32(&log->head, next);
    publisher->next = next;
    return n;
}

/* The cursor of the oldest retained entry, from `last`, the stamp of the last slot,
 * and `head`, the head loaded after it (see sce_blogn_reader_oldest for why the
 * stamp decides). `head` is read only when `last` is not SCE_BLOGN_UNWRITTEN, so a
 * caller that has not loaded it may pass anything then. A reader opened at the
 * oldest and a deferred reader binding at its first read take the cursor from
 * here, so they cannot disagree. */
static inline uint32_t sce_blogn_oldest_cursor(const sce_blogn_t *log, uint32_t last, uint32_t head) {
    return last == SCE_BLOGN_UNWRITTEN ? 0u : sce_blogn_distance(head, sce_blogn_retention(log));
}

/* A reader at the oldest retained entry. The oldest retained entry is
 * `max(0, h - retention)`, and a count modulo 2^w cannot tell a log that has
 * published fewer than `retention` entries from one that wrapped, so the log says
 * it itself: the last slot still holds SCE_BLOGN_UNWRITTEN before the publish of
 * entry `retention` begins. If it does, at most `retention` entries had been
 * published when it was loaded and the oldest is entry 0, whose cursor is 0. If it
 * does not, `h >= retention`, and the oldest is `h - retention` of the head loaded
 * after. */
static inline sce_blogn_reader_t sce_blogn_reader_oldest(const sce_blogn_t *log) {
    sce_blogn_reader_t reader;
    const uint32_t last = sce_atomic_load_acquire_u32(sce_blogn_slot(log, log->slots - 1u));
    reader.cursor = sce_blogn_oldest_cursor(log, last, last == SCE_BLOGN_UNWRITTEN ? 0u : sce_blogn_head(log));
    reader.lost = 0;
    reader.unbound = 0u;
    return reader;
}

/* A reader whose start is the oldest retained entry at its first read, and not at
 * this call: it has no cursor until sce_blogn_read binds one, from the head that
 * read loads, so a reader that waited is told of nothing the publisher did
 * meanwhile, where sce_blogn_reader_oldest would tell it of the lag. Opening it
 * loads nothing (`log` is not touched) and costs the log nothing.
 *
 * The first read binds as sce_blogn_reader_oldest does, with its loads part of the
 * read: the stamp of the last slot, then the head (the order of the models:
 * `RDStart`, `RDHead`), the head being the one the read goes on with. It then
 * answers as any read does: an item, empty for a log with nothing published, or
 * lagged if the publisher overtook the cursor inside that read. The deploy's pause
 * bound covers the two loads as it covers the loads of a read, counted from the
 * first of them (ADR 0062 section 2). */
static inline sce_blogn_reader_t sce_blogn_reader_deferred(const sce_blogn_t *log) {
    sce_blogn_reader_t reader;
    (void)log;
    reader.cursor = 0u;
    reader.lost = 0;
    reader.unbound = 1u;
    return reader;
}

/* Whether `reader` has a cursor: zero only for a deferred reader that has not
 * read yet. */
static inline int sce_blogn_reader_is_bound(const sce_blogn_reader_t *reader) {
    return reader->unbound == 0u;
}

/* A reader at the head: it sees only what is published after this call. */
static inline sce_blogn_reader_t sce_blogn_reader_head(const sce_blogn_t *log) {
    sce_blogn_reader_t reader;
    reader.cursor = sce_blogn_head(log);
    reader.lost = 0;
    reader.unbound = 0u;
    return reader;
}

/* A reader at the sequence number `seq`, or at the head when `seq` is past it. A
 * log knows a position only modulo 2^w, so `seq` is reduced to it and read as an
 * entry the caller got from this log (a publish's return or an item's `seq`):
 * fewer than K = SCE_BLOGN_HALF entries behind the head it is taken as it is, and
 * any other position (K or more behind, which is what one ahead of the head looks
 * like) is the head. */
static inline sce_blogn_reader_t sce_blogn_reader_at(const sce_blogn_t *log, uint64_t seq) {
    sce_blogn_reader_t reader;
    const uint32_t head = sce_blogn_head(log);
    const uint32_t residue = (uint32_t)(seq & (uint64_t)SCE_BLOGN_MASK);
    reader.cursor = sce_blogn_distance(head, residue) < SCE_BLOGN_HALF ? residue : head;
    reader.lost = 0;
    return reader;
}

/* Moves `reader` to the oldest entry retained and says how far that was, from
 * `behind`, how far behind the head the reader found itself. A lag is only
 * answered once the publisher is `slots` entries or more ahead, so a `behind`
 * below that came from a head that has wrapped since the reader's cursor was
 * valid: one lap of the stamp space is added, the fewest it can be. The cursor
 * lands on the oldest retained entry's residue whatever the number of laps; only
 * the count is then short. */
static inline sce_blogn_read_t sce_blogn_lagged(sce_blogn_reader_t *reader, const sce_blogn_t *log, uint32_t behind) {
    sce_blogn_read_t result;
    const uint64_t retention = (uint64_t)sce_blogn_retention(log);
    uint64_t distance = (uint64_t)behind;
    uint64_t skipped;
    if (distance < (uint64_t)log->slots) {
        distance += SCE_BLOGN_SPACE;
    }
    skipped = distance > retention ? distance - retention : 0u;
    reader->cursor = sce_blogn_advance(reader->cursor, skipped);
    reader->lost = skipped > UINT64_MAX - reader->lost ? UINT64_MAX : reader->lost + skipped;
    result.status = SCE_BLOGN_LAGGED;
    result.seq = 0;
    result.skipped = skipped;
    return result;
}

/* Reads the entry at `reader`'s cursor into `out` (`words` 32-bit words). A
 * bounded number of steps whatever the publisher is doing: a copy that the
 * publisher overwrote is not retried, it is the answer (SCE_BLOGN_LAGGED). */
static inline sce_blogn_read_t sce_blogn_read(const sce_blogn_t *log, sce_blogn_reader_t *reader, uint32_t *out) {
    sce_blogn_read_t result;
    uint32_t r;
    uint32_t head;
    uint32_t behind;
    if (reader->unbound != 0u) {
        /* The first read of a deferred reader binds its cursor first: the stamp of
         * the last slot, then the head, which is the head the read goes on with, so
         * the read loads it once. */
        const uint32_t last = sce_atomic_load_acquire_u32(sce_blogn_slot(log, log->slots - 1u));
        head = sce_atomic_load_acquire_u32(&log->head);
        reader->cursor = sce_blogn_oldest_cursor(log, last, head);
        reader->unbound = 0u;
    } else {
        head = sce_atomic_load_acquire_u32(&log->head);
    }
    r = reader->cursor;
    behind = sce_blogn_distance(head, r);
    result.seq = 0;
    result.skipped = 0;
    if (behind == 0u) {
        result.status = SCE_BLOGN_EMPTY;
        return result;
    }
    if (behind >= log->slots) {
        return sce_blogn_lagged(reader, log, behind);
    }
    {
        /* `r` was in the window when `head` was loaded, so its slot was filled
         * before that load. It holds `r` still unless the publisher has since
         * begun writing `r + slots`. */
        const uint32_t *slot = sce_blogn_slot(log, r);
        const uint32_t first = sce_atomic_load_acquire_u32(&slot[0]);
        if (first == sce_blogn_done(r)) {
            uint32_t k;
            for (k = 0; k < log->words; k++) {
                out[k] = sce_atomic_load_relaxed_u32(&slot[1u + k]);
            }
            /* The fence keeps the copy before the second load, which is what makes
             * an unchanged stamp mean an unchanged copy. The load is an acquire for
             * the other reason: if it sees the stamp of a later lap, the reload of
             * `head` below must see the publish that began that lap. A fence orders
             * a later load only against reads before it, so this load cannot rely
             * on the fence for that, and a relaxed one reports a lag that skipped
             * nothing. */
            sce_atomic_fence_acquire();
            if (sce_atomic_load_acquire_u32(&slot[0]) == first) {
                reader->cursor = (uint32_t)sce_blogn_advance(r, 1u);
                result.status = SCE_BLOGN_ITEM;
                result.seq = r;
                return result;
            }
        }
    }
    /* The slot was reused while it was being read, so the publisher has published
     * through `r + retention` and `head` shows it. */
    head = sce_atomic_load_acquire_u32(&log->head);
    return sce_blogn_lagged(reader, log, sce_blogn_distance(head, r));
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_FORGE_BROADCAST_LOG_NARROW_H */
