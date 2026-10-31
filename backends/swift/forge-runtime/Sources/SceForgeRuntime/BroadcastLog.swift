// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runtime half of `sce:kind="broadcast-log"` (SCE Protocol-Synthesis RFC §synth-5-S, ADR 0062)
// for Swift: a bounded log with one publisher and any number of readers, which forgets its oldest
// entry when it is full. Siblings: `com.sce.forge.runtime.broadcastlog.BroadcastLog` (Kotlin,
// `backends/kotlin/forge-runtime/.../broadcastlog/BroadcastLog.kt`) and the Rust row it is written
// from (`backends/rust/forge-runtime/src/broadcast_log_core.rs`, instantiated at 64 bits).
//
// The contract is the sequential log of ADR 0062 section 2. `h` entries have been published and
// only the last `retention` are retained. A reader owns a cursor: a read returns the entry at the
// cursor, or says the log has nothing newer, or says how many entries the reader missed and moves
// it to the oldest retained. Publishing never fails and never waits, and neither does a read: both
// are wait-free, in a number of steps that depends on the entry's width and on nothing the other
// side does.
//
// **The protocol** (ADR 0062 section 4), access for access the Rust row's. A slot holds a stamp and
// the entry's words. The publisher writes a slot as `stamp := busy(n)` (release), a release fence,
// the words (relaxed), `stamp := done(n)` (release), then `head := n + 1` (release). A reader
// holding cursor `r` loads `head` (acquire) and answers `empty` at the head and `lagged` outside
// the window; otherwise it loads the stamp of slot `r mod slots` (acquire), copies the words
// (relaxed), runs an acquire fence, and loads the stamp again (acquire): if it is still `done(r)`
// the copy is entry `r`; if not, the publisher has reused the slot, and the reader reloads `head`
// (acquire) and answers `lagged`. It is a sequence lock in the form Boehm showed correct under the
// C++11 memory model, which the `Synchronization` atomics of Swift 6 follow: the orderings below
// are the ones the loom and GenMC models of the Rust row check, and each is justified at the access
// that carries it in `broadcast_log_core.rs`. A failed validation is the answer and is not retried,
// which is what makes a read wait-free.
//
// **The slot count is `retention + 1`.** The slot being overwritten is in flux and cannot be
// promised to a reader.
//
// **Only the 64-bit row.** A stamp, the head and every word are `Atomic<UInt64>`, so the count
// never wraps (the publisher stops at `BroadcastLog.horizon`) and the narrow row of the Rust
// runtime (a 32-bit stamp, for a target with no 64-bit atomic) has no Swift counterpart: the
// generator refuses a deploy that asks for it.
//
// What differs from the languages with constant generics: the width and the slot count are
// arguments of the initializer, and the storage is allocated once, by it, and never again:
// publishing allocates nothing.

import Synchronization

/// What a read found.
public enum BroadcastLogRead: Equatable, Sendable {
    /// The entry `seq` was copied into the caller's buffer, and the reader now holds `seq + 1`.
    case item(seq: UInt64)

    /// The reader is at the head: nothing newer is published. Its cursor is unchanged.
    case empty

    /// The reader had fallen behind the window and missed `skipped >= 1` entries. It now holds
    /// the oldest retained entry, so its next read returns that. The caller's buffer is
    /// unspecified.
    case lagged(skipped: UInt64)
}

/// A reader's position, owned by the reader and shared with nothing. Opening one costs the log
/// nothing, which is what keeps publishing wait-free. A reader is for one thread at a time: its
/// cursor is plain state, moved by `BroadcastLog.read`.
public struct BroadcastLogReader: Equatable, Sendable {
    /// The sequence number this reader reads next. A reader opened by
    /// `BroadcastLog.readerDeferred()` has none until its first read binds one: this is 0 and
    /// `isBound` is false until then.
    public internal(set) var cursor: UInt64

    /// The entries this reader has been told it missed, summed over its reads.
    public internal(set) var lost: UInt64 = 0

    /// Whether the reader has a cursor: false only for a deferred reader that has not read yet.
    public internal(set) var isBound: Bool

    init(cursor: UInt64, isBound: Bool = true) {
        self.cursor = cursor
        self.isBound = isBound
    }
}

/// A bounded log whose entries are `words` 64-bit words, kept in `slots` slots and retaining the
/// last `slots - 1` entries (ADR 0062). One publisher, any number of readers.
///
/// **Storage.** One allocation of `Atomic<UInt64>` cells, made by the initializer and released by
/// the deinitializer: a header of `headerCells` cells whose first is the head (the rest keep the
/// first slot off the head's cache line, which the publisher writes at every publish and every
/// reader loads), then `slots` slots of one stamp cell and `words` word cells each. `Atomic` is
/// noncopyable and may not be a `var`, so it cannot sit in an `Array`; a pointer to cells that are
/// initialized in place, one by one, is what holds a run of them.
///
/// **Sendable.** The class is `@unchecked Sendable`, and that is sound: after the initializer it
/// mutates nothing but the cells, every access to a cell is an atomic operation, and the one fact
/// the protocol assumes of its writer (a single publisher) is what `publisher()` enforces with
/// `publisherTaken`. The pointer is never reallocated and never escapes the class.
public final class BroadcastLog: @unchecked Sendable {
    /// The most entries a log can publish: the stamps are `2n + 2`, which must fit a `UInt64`.
    /// Past it a publish would corrupt the stamps, so `BroadcastLogPublisher.publish` stops there
    /// instead of publishing. It is `2^63 - 2`: 292 years at a billion publishes per second.
    public static let horizon: UInt64 = (UInt64.max - 2) / 2

    /// The cells before the first slot: the head, and the padding that keeps the head's cache line
    /// to itself on a target with lines of up to 128 bytes.
    private static let headerCells = 16

    /// The words of an entry.
    public let words: Int

    /// The slots the log keeps: one more than the retention.
    public let slots: Int

    /// The entries a reader may rely on.
    public let retention: UInt64

    private let cells: UnsafeMutablePointer<Atomic<UInt64>>
    private let cellCount: Int

    /// Whether a `BroadcastLogPublisher` exists.
    fileprivate let publisherTaken = Atomic<Bool>(false)

    /// An empty log of `words`-word entries kept in `slots` slots. A `slots` below two (a log
    /// retains at least one entry), a negative `words` and a size whose cells would not be
    /// countable stop the program: they are a defect of the caller and not an input.
    public init(words: Int, slots: Int) {
        precondition(slots >= 2, "a log retains at least one entry, so it has at least two slots")
        precondition(words >= 0, "an entry has a number of words, not a negative one")
        let (stride, strideOverflow) = words.addingReportingOverflow(1)
        let (slotCells, slotOverflow) = slots.multipliedReportingOverflow(by: stride)
        let (total, totalOverflow) = slotCells.addingReportingOverflow(Self.headerCells)
        precondition(
            !(strideOverflow || slotOverflow || totalOverflow),
            "a log of \(slots) slots of \(words) words has more cells than can be counted")
        let storage = UnsafeMutablePointer<Atomic<UInt64>>.allocate(capacity: total)
        for index in 0..<total {
            (storage + index).initialize(to: Atomic<UInt64>(0))
        }
        self.words = words
        self.slots = slots
        self.retention = UInt64(slots - 1)
        self.cells = storage
        self.cellCount = total
    }

    deinit {
        cells.deinitialize(count: cellCount)
        cells.deallocate()
    }

    /// The stamp cell of slot `slot`.
    @inline(__always)
    private func stampCell(_ slot: Int) -> UnsafeMutablePointer<Atomic<UInt64>> {
        cells + (Self.headerCells + slot * (words + 1))
    }

    /// The cell of word `index` of slot `slot`.
    @inline(__always)
    private func wordCell(_ index: Int, of slot: Int) -> UnsafeMutablePointer<Atomic<UInt64>> {
        cells + (Self.headerCells + slot * (words + 1) + 1 + index)
    }

    /// The number of entries published so far. Acquire, as in the Rust row.
    public var head: UInt64 {
        cells.pointee.load(ordering: .acquiring)
    }

    /// The publisher, or `nil` while another exists. Drop it, or call its `release()`, to give
    /// the place back; its successor continues the sequence.
    public func publisher() -> BroadcastLogPublisher? {
        // Acquire, as in the Rust row.
        if publisherTaken.exchange(true, ordering: .acquiring) {
            return nil
        }
        return BroadcastLogPublisher(log: self, next: head)
    }

    /// A reader at the oldest retained entry.
    public func readerOldest() -> BroadcastLogReader {
        // A 64-bit count does not wrap, so the oldest retained entry is the head less the
        // retention, no lower than 0: one load.
        let current = head
        return BroadcastLogReader(cursor: current > retention ? current - retention : 0)
    }

    /// A reader whose start is the oldest retained entry **at its first read**, and not at this
    /// call: it has no cursor until that read binds one, from the head that read loads, so a
    /// reader that waited is told of nothing the publisher did meanwhile, where `readerOldest()`
    /// would tell it of the lag. Opening it loads nothing and costs the log nothing.
    ///
    /// The first read binds as `readerOldest()` does and then goes on as any read does: the answer
    /// is an item, `empty` for a log with nothing published, or `lagged` if the publisher
    /// overtook the cursor inside that read (ADR 0062 section 2).
    public func readerDeferred() -> BroadcastLogReader {
        BroadcastLogReader(cursor: 0, isBound: false)
    }

    /// A reader at the head: it sees only what is published after this call.
    public func readerHead() -> BroadcastLogReader {
        BroadcastLogReader(cursor: head)
    }

    /// A reader at the sequence number `seq`, or at the head when `seq` is past it.
    public func readerAt(_ seq: UInt64) -> BroadcastLogReader {
        BroadcastLogReader(cursor: min(seq, head))
    }

    /// Reads the entry at `reader`'s cursor into `out`, which holds `words` words. The cursor
    /// moves to `seq + 1` on `item`, stays on `empty`, and moves to the oldest retained entry on
    /// `lagged`, adding what it skipped to the reader's `lost`. `out` is unspecified unless the
    /// result is an `item`.
    ///
    /// A bounded number of steps whatever the publisher is doing: a copy that the publisher
    /// overwrote is not retried, it is the answer (`lagged`).
    public func read(_ reader: inout BroadcastLogReader, into out: inout [UInt64]) -> BroadcastLogRead {
        precondition(out.count == words, "the buffer holds \(out.count) words and an entry is \(words)")
        // The first read of a deferred reader binds its cursor at the head it loads here, and
        // then goes on as every read does with that same head. Acquire, as in the Rust row.
        let current = reader.isBound ? cells.pointee.load(ordering: .acquiring) : bind(&reader)
        let r = reader.cursor
        let behind = current &- r
        if behind == 0 {
            return .empty
        }
        if behind >= UInt64(slots) {
            return lagged(&reader, behind: behind)
        }
        // `r` was in the window when `head` was loaded, so its slot was filled before that load.
        // It holds `r` still unless the publisher has since begun writing `r + slots`.
        let slot = Int(r % UInt64(slots))
        let stamp = stampCell(slot)
        // Acquire, as in the Rust row.
        let first = stamp.pointee.load(ordering: .acquiring)
        if first == Self.done(r) {
            for index in 0..<words {
                // Relaxed, as in the Rust row.
                out[index] = wordCell(index, of: slot).pointee.load(ordering: .relaxed)
            }
            // The fence keeps the copy before the second load, which is what makes an unchanged
            // stamp mean an unchanged copy. The load is an acquire for the other reason: if it
            // sees the stamp of a later lap, the reload of `head` below must see the publish that
            // began that lap, and a fence orders a later load only against reads before it.
            atomicMemoryFence(ordering: .acquiring)
            if stamp.pointee.load(ordering: .acquiring) == first {
                reader.cursor = r + 1
                return .item(seq: r)
            }
        }
        // The slot was reused while it was being read, so the publisher has published through
        // `r + retention` and `head` shows it. Acquire, as in the Rust row.
        let reloaded = cells.pointee.load(ordering: .acquiring)
        return lagged(&reader, behind: reloaded &- r)
    }

    /// Binds a deferred `reader`'s cursor to the oldest retained entry, as `readerOldest()` does,
    /// and returns the head the cursor was taken from, which is the load the read goes on with.
    /// A 64-bit count does not wrap, so the one load of the head is all the bind needs.
    private func bind(_ reader: inout BroadcastLogReader) -> UInt64 {
        // Acquire, as in the Rust row.
        let current = cells.pointee.load(ordering: .acquiring)
        reader.cursor = current > retention ? current - retention : 0
        reader.isBound = true
        return current
    }

    /// Moves `reader` to the oldest entry retained and says how far that was, from `behind`, how
    /// far behind the head the reader found itself.
    private func lagged(_ reader: inout BroadcastLogReader, behind: UInt64) -> BroadcastLogRead {
        let skipped = behind > retention ? behind - retention : 0
        assert(skipped >= 1, "a lag skips at least one entry")
        reader.cursor = reader.cursor &+ skipped
        let (sum, overflow) = reader.lost.addingReportingOverflow(skipped)
        reader.lost = overflow ? UInt64.max : sum
        return .lagged(skipped: skipped)
    }

    /// The stamp a slot holds while the entry `n` is being written into it.
    private static func busy(_ n: UInt64) -> UInt64 { 2 &* n &+ 1 }

    /// The stamp a slot holds once the entry `n` is in it.
    private static func done(_ n: UInt64) -> UInt64 { 2 &* n &+ 2 }

    /// Writes `entry` as the entry `n`: the publisher's half of the protocol.
    fileprivate func write(_ entry: [UInt64], as n: UInt64) {
        let slot = Int(n % UInt64(slots))
        let stamp = stampCell(slot)
        // Release, for the reader that sees this stamp and then reloads `head`: it must find
        // `head` at `n` or later, and the store of `head` to `n` ended the previous publish, in
        // this thread's order before this one.
        stamp.pointee.store(Self.busy(n), ordering: .releasing)
        // A release store orders what came before it, and this stamp must also come before the
        // words.
        atomicMemoryFence(ordering: .releasing)
        for index in 0..<words {
            // Relaxed, as in the Rust row.
            wordCell(index, of: slot).pointee.store(entry[index], ordering: .relaxed)
        }
        // Release, as in the Rust row.
        stamp.pointee.store(Self.done(n), ordering: .releasing)
        // The linearization point: a reader that loads this index (acquire) sees the slot's stamp
        // and words. Release, as in the Rust row.
        cells.pointee.store(n + 1, ordering: .releasing)
    }

    /// Sets the number of entries published to `published` without writing any of them, so a test
    /// can stand at the `horizon`, which 2^63 publishes would take centuries to reach. The slots
    /// stay empty, so only a publisher is meaningful afterwards; a reader below `published` would
    /// be told what the protocol cannot say. Not for use while a publisher exists.
    func startAt(published: UInt64) {
        precondition(published <= Self.horizon, "the head is within the horizon")
        precondition(
            !publisherTaken.load(ordering: .acquiring), "the head is set while no publisher exists")
        cells.pointee.store(published, ordering: .releasing)
    }
}

/// The publishing side of a `BroadcastLog`. There is one at a time, and it is for one thread at a
/// time.
///
/// It holds its place until it is released: `release()` gives the place back, and so does dropping
/// the last reference to it. The class is `@unchecked Sendable` so that a publisher can be handed
/// to the thread that publishes: its own count is plain state that one thread at a time touches,
/// and the hand-over is the caller's synchronization.
public final class BroadcastLogPublisher: @unchecked Sendable {
    private var log: BroadcastLog?

    /// The sequence number of the next entry: this side's own count, so publishing reads no
    /// shared state.
    private var next: UInt64

    fileprivate init(log: BroadcastLog, next: UInt64) {
        self.log = log
        self.next = next
    }

    deinit {
        release()
    }

    /// Publishes `entry`, which holds the log's `words` words, and returns the sequence number it
    /// took. Never fails and never waits. Stops the program past `BroadcastLog.horizon` entries,
    /// which is not reached in practice, and when the publisher was released.
    @discardableResult
    public func publish(_ entry: [UInt64]) -> UInt64 {
        guard let owner = log else {
            preconditionFailure("the publisher was released")
        }
        precondition(
            entry.count == owner.words,
            "the entry holds \(entry.count) words and the log's is \(owner.words)")
        let n = next
        precondition(n < BroadcastLog.horizon, "the log has published its horizon of entries")
        owner.write(entry, as: n)
        next = n + 1
        return n
    }

    /// Gives the publisher's place back. Releasing a publisher that is released does nothing, so a
    /// place that a successor has taken is not given back a second time.
    public func release() {
        guard let owner = log else {
            return
        }
        log = nil
        // Release, as in the Rust row.
        owner.publisherTaken.store(false, ordering: .releasing)
    }
}
