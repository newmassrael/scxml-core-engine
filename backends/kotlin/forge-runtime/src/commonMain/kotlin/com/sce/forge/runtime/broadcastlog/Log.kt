// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * The runtime half of `sce:kind="broadcast-log"` (SCE Protocol-Synthesis RFC
 * §synth-5-S, ADR 0062) for Kotlin: a bounded log with one publisher and any number of readers, which forgets its
 * oldest entry when it is full.
 *
 * The contract is the sequential log of ADR 0062 section 2. `h` entries have
 * been published and only the last `retention` are retained. A reader owns a
 * cursor: a read returns the entry at the cursor, or says the log has nothing
 * newer, or says how many entries the reader missed and moves it to the oldest
 * retained. Publishing never fails and never waits, and neither does a read:
 * both are wait-free, in a number of steps that depends on the entry's width
 * and on nothing the other side does.
 *
 * - [BroadcastLog] is the log: a ring of `retention + 1` stamped slots read
 *   with a sequence lock (ADR 0062 section 4). It is the same protocol as the
 *   Rust row (`backends/rust/forge-runtime/src/broadcast_log.rs`), access for
 *   access.
 * - [Reader] is a reader's position, owned by the reader and shared with
 *   nothing. Opening one costs the log nothing, which is what keeps publishing
 *   wait-free.
 * - [Read] is what a read found.
 *
 * The atomics are `kotlin.concurrent.atomics`, which on the JVM are
 * `java.util.concurrent`'s: every access is sequentially consistent. The Rust
 * row asks for acquire and release accesses and two fences, and a sequentially
 * consistent access is at least as strong as each of them, so the accesses
 * here are the Rust row's in the same order and the fences are not needed:
 * `commonMain` has no fence, and a fence between two sequentially consistent
 * accesses orders nothing they do not already order. Each access in
 * [BroadcastLog] says which Rust order it stands for.
 *
 * The stamp is 64-bit. A sequence number is unsigned in the contract and a
 * `Long` is signed, so the sequence stops at [HORIZON], which is where
 * `2n + 2` last fits in the positive half. Everything the log holds stays
 * non-negative, and a publisher at the horizon throws instead of publishing.
 *
 * What differs from the languages with destructors and constant generics:
 *
 * - The width and the slot count are arguments of the constructor. The storage
 *   is allocated by the constructor and never again: publishing allocates
 *   nothing. A read allocates the one small result object it returns.
 * - An entry is `words` 64-bit words, copied into the caller's `LongArray`.
 * - The publisher holds its place until `release()` is called. There is no
 *   destructor to give the place back, so the caller does, usually in a
 *   `finally`.
 */
package com.sce.forge.runtime.broadcastlog

/**
 * The most entries a log can publish: the stamps are `2n + 2`, which must fit
 * a positive `Long`. Past it a publish would corrupt the stamps, so
 * [BroadcastLog.Publisher.publish] stops there with an exception instead of
 * publishing. It is `2^62 - 2`: 146 years at a billion publishes per second.
 */
public const val HORIZON: Long = (Long.MAX_VALUE - 2) / 2

/** What a read found. */
public sealed interface Read {
    /**
     * The entry [seq] was copied into the caller's buffer, and the reader now
     * holds `seq + 1`.
     */
    public data class Item(public val seq: Long) : Read

    /** The reader is at the head: nothing newer is published. Its cursor is unchanged. */
    public data object Empty : Read

    /**
     * The reader had fallen behind the window and missed `skipped >= 1`
     * entries. It now holds the oldest retained entry, so its next read
     * returns that. The caller's buffer is unspecified.
     */
    public data class Lagged(public val skipped: Long) : Read
}

/**
 * A reader's position, owned by the reader and shared with nothing. A reader
 * is for one thread at a time: its cursor is plain state, moved by
 * [BroadcastLog.read].
 */
public class Reader internal constructor(cursor: Long, bound: Boolean = true) {
    /**
     * The sequence number this reader reads next. A reader opened by
     * [BroadcastLog.readerDeferred] has none until its first read binds one:
     * this is 0 and [isBound] is false until then.
     */
    public var cursor: Long = cursor
        internal set

    /**
     * Whether the reader has a cursor: false only for a deferred reader that has
     * not read yet.
     */
    public var isBound: Boolean = bound
        internal set

    /** The entries this reader has been told it missed, summed over its reads. */
    public var lost: Long = 0
        internal set
}
