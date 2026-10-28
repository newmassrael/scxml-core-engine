// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.forge.runtime.broadcastlog

import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/** The stamp a slot holds while the entry [n] is being written into it. */
private fun busy(n: Long): Long = 2 * n + 1

/** The stamp a slot holds once the entry [n] is in it. */
private fun done(n: Long): Long = 2 * n + 2

/** A slot: a stamp and the entry's words. */
private class Slot(width: Int) {
    val stamp = AtomicLong(0)
    val words: Array<AtomicLong> = Array(width) { AtomicLong(0) }
}

/**
 * A bounded log whose entries are [words] 64-bit words, kept in [slots] slots
 * and retaining the last `slots - 1` entries (ADR 0062). One publisher, any
 * number of readers.
 *
 * **The protocol** (ADR 0062 section 4), as in the Rust row
 * (`backends/rust/forge-runtime/src/broadcast_log.rs`). A slot holds a stamp
 * and the entry's words. The publisher writes a slot as `stamp := busy(n)`, the
 * words, `stamp := done(n)`, then `head := n + 1`. A reader holding cursor `r`
 * loads `head` and answers [Read.Empty] at the head and [Read.Lagged] outside
 * the window; otherwise it loads the stamp of slot `r mod slots`, copies the
 * words, and loads the stamp again: if it is still `done(r)` the copy is entry
 * `r`; if not, the publisher has reused the slot and the reader reloads `head`
 * and answers [Read.Lagged]. It is a sequence lock, and the stamp is not
 * replaced by a lock here: a failed validation is the answer and is not
 * retried, which is what makes a read wait-free.
 *
 * **Ordering.** Every access is sequentially consistent (see the package
 * documentation). The comments name the order the Rust row uses for the same
 * access, which this one is at least as strong as. The two fences of the Rust
 * row have no counterpart: between sequentially consistent accesses they order
 * nothing that is not ordered already, so the copy of the words lies between
 * the two stamp loads, and the busy stamp precedes the words, in the one total
 * order every thread agrees on.
 *
 * **The slot count is `retention + 1`.** The slot being overwritten is in flux
 * and cannot be promised to a reader. With `retention` slots a reader could
 * find the entry the specification retains already being overwritten and be
 * told it lagged by nothing (`docs/formal/BroadcastLog.tla` finds that state).
 *
 * The constructor refuses a [slots] below two and a [words] below zero by
 * throwing `IllegalArgumentException`.
 */
public class BroadcastLog(public val words: Int, public val slots: Int) {
    /** The entries a reader may rely on. */
    public val retention: Long = (slots - 1).toLong()

    /**
     * The number of entries published. Written only by the publisher, loaded by
     * every reader.
     */
    private val head = AtomicLong(0)

    /** Whether a [Publisher] exists. */
    private val publisherTaken = AtomicBoolean(false)

    private val ring: Array<Slot>

    init {
        require(slots >= 2) { "a log retains at least one entry, so it has at least two slots" }
        require(words >= 0) { "an entry has a number of words, not a negative one" }
        ring = Array(slots) { Slot(words) }
    }

    /** The slot sequence number [n] lives in. */
    private fun slotOf(n: Long): Slot = ring[(n % slots.toLong()).toInt()]

    /** The number of entries published so far. Acquire in the Rust row. */
    public fun head(): Long = head.load()

    /**
     * The publisher, or `null` while another exists. Release the handle to give
     * the place back; its successor continues the sequence.
     */
    public fun publisher(): Publisher? {
        // Acquire, in the Rust row.
        if (publisherTaken.exchange(true)) {
            return null
        }
        return Publisher(this, head.load())
    }

    /** A reader at the oldest retained entry. */
    public fun readerOldest(): Reader = Reader(maxOf(head() - retention, 0L))

    /** A reader at the head: it sees only what is published after this call. */
    public fun readerHead(): Reader = Reader(head())

    /**
     * A reader at the sequence number [seq], or at the head when [seq] is past
     * it. A sequence number is not negative.
     */
    public fun readerAt(seq: Long): Reader {
        require(seq >= 0) { "a sequence number is not negative" }
        return Reader(minOf(seq, head()))
    }

    /**
     * Reads the entry at [reader]'s cursor into [out], which holds [words]
     * words. The cursor moves to `seq + 1` on [Read.Item], stays on
     * [Read.Empty], and moves to the oldest retained entry on [Read.Lagged],
     * adding what it skipped to the reader's `lost`. [out] is unspecified
     * unless the result is an [Read.Item].
     *
     * A bounded number of steps whatever the publisher is doing: a copy that
     * the publisher overwrote is not retried, it is the answer
     * ([Read.Lagged]).
     */
    public fun read(reader: Reader, out: LongArray): Read {
        require(out.size == words) { "the buffer holds ${out.size} words and an entry is $words" }
        val r = reader.cursor
        // Acquire, in the Rust row.
        val h = head.load()
        assert(r <= h) { "a reader's cursor never passes the head" }
        if (r >= h) {
            return Read.Empty
        }
        if (h - r >= slots.toLong()) {
            return lagged(reader, h)
        }
        // `r` was in the window when `head` was loaded, so its slot was filled
        // before that load. It holds `r` still unless the publisher has since
        // begun writing `r + slots`.
        val slot = slotOf(r)
        // Acquire, in the Rust row.
        val first = slot.stamp.load()
        if (first == done(r)) {
            for (k in 0 until words) {
                // Relaxed, in the Rust row.
                out[k] = slot.words[k].load()
            }
            // The Rust row puts an acquire fence here, keeping the copy before
            // this second load, which is what makes an unchanged stamp mean an
            // unchanged copy. The load is an acquire in the Rust row for the
            // other reason: if it sees the stamp of a later lap, the reload of
            // `head` below must see the publish that began that lap.
            if (slot.stamp.load() == first) {
                reader.cursor = r + 1
                return Read.Item(r)
            }
        }
        // The slot was reused while it was being read, so the publisher has
        // published through `r + retention` and `head` shows it. Acquire, in
        // the Rust row.
        return lagged(reader, head.load())
    }

    /**
     * Moves [reader] to the oldest entry retained when the head is [h], and
     * says how far that was.
     */
    private fun lagged(reader: Reader, h: Long): Read {
        val oldest = maxOf(h - retention, 0L)
        val skipped = maxOf(oldest - reader.cursor, 0L)
        assert(skipped >= 1) { "a lag skips at least one entry" }
        reader.cursor = oldest
        reader.lost = if (reader.lost > Long.MAX_VALUE - skipped) Long.MAX_VALUE else reader.lost + skipped
        return Read.Lagged(skipped)
    }

    /**
     * Sets the number of entries published to [published] without writing any
     * of them, so a test can stand at the [HORIZON], which 2^62 publishes
     * would take years to reach. The slots stay empty, so only a publisher is
     * meaningful afterwards; a reader below [published] would be told what the
     * protocol cannot say. Not for use while a publisher exists.
     */
    internal fun startAt(published: Long) {
        require(published in 0..HORIZON) { "the head is within the horizon" }
        check(!publisherTaken.load()) { "the head is set while no publisher exists" }
        head.store(published)
    }

    /**
     * The publishing side of a [BroadcastLog]. There is one at a time, and it is
     * for one thread at a time.
     */
    public class Publisher internal constructor(
        private var log: BroadcastLog?,
        /**
         * The sequence number of the next entry: this side's own count, so
         * publishing reads no shared state.
         */
        private var next: Long,
    ) {
        /**
         * Publishes [entry], which holds the log's [BroadcastLog.words] words,
         * and returns the sequence number it took. Never fails and never waits.
         * Throws `IllegalStateException` past [HORIZON] entries, which is not
         * reached in practice.
         */
        public fun publish(entry: LongArray): Long {
            val owner = checkNotNull(log) { "the publisher was released" }
            require(entry.size == owner.words) { "the entry holds ${entry.size} words and the log's is ${owner.words}" }
            val n = next
            check(n < HORIZON) { "the log has published its horizon of entries" }
            val slot = owner.slotOf(n)
            // Release, in the Rust row, for the reader that sees this stamp and
            // then reloads `head`: it must find `head` at `n` or later, and the
            // store of `head` to `n` ended the previous publish, in this
            // thread's order before this one.
            slot.stamp.store(busy(n))
            // The Rust row puts a release fence here, so the busy stamp comes
            // before the words.
            for (k in 0 until owner.words) {
                // Relaxed, in the Rust row.
                slot.words[k].store(entry[k])
            }
            // Release, in the Rust row.
            slot.stamp.store(done(n))
            // The linearization point: a reader that loads this index sees the
            // slot's stamp and words. Release, in the Rust row.
            owner.head.store(n + 1)
            next = n + 1
            return n
        }

        /** Gives the publisher place back. The handle is not usable afterwards. */
        public fun release() {
            // Release, in the Rust row.
            log?.publisherTaken?.store(false)
            log = null
        }
    }
}
