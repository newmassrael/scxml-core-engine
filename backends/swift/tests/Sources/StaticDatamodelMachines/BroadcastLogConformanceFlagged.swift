// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d95af6fbc211ea1c5b1e249d16b701ebad41657a9f4b213349f1c668ce4ccbb
// SCE-MAP: broadcast_log_conformance_flagged.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// ADR 0062 broadcast-log — a bounded log that one execution context appends
// to and any number of readers read, each by a cursor of its own. The document
// states the contract (element type `broadcast_log_conformance_flagged_event`, one producer,
// `wait-free`, the last 3 entries retained) and names no algorithm;
// the algorithm is the runtime's sequence-locked ring.
//
// An entry is the element's wire bytes, never a value: the log stores the
// bytes the element document's codec writes, so a copy that was torn is thrown
// away unread, and the element a reader gets is a value of its own, decoded
// from a buffer only that read holds.
//
// Operations (from `SceForgeRuntime`, typed here over the element):
//   BroadcastLogConformanceFlagged.publish(publisher, element, flags: flags) -> BroadcastLogConformanceFlagged.Publish
//       wait-free; the entry's sequence number, or why the element was refused
//   BroadcastLogConformanceFlagged.read(log, &reader) -> BroadcastLogConformanceFlagged.Read
//       wait-free; an item, nothing new, how many were missed, or an entry that does not decode
// A reader is `log.readerOldest()`, `log.readerHead()`, `log.readerAt(n)` or
// `log.readerDeferred()` and owns its cursor; the publisher is `log.publisher()`,
// taken once and given back by `release()` or by dropping it.
//
// The element's codec asks its caller for 1 flag input(s). Those are
// part of the entry: the log stores them in the entry's context words beside the
// bytes, so every reader decodes an entry under what it was published under (ADR 0062
// section 4).
// `publish` takes the flags and an `item` returns them.
//
// The names are the members of the enum `BroadcastLogConformanceFlagged` and not top-level
// declarations: every unit of a SwiftPM target shares one module, so the
// constants of two logs would otherwise be one redeclared name.

import SceForgeRuntime

/// The log of `broadcast_log_conformance_flagged_event` entries: its constants, its typed `publish` and
/// `read`, and the results they answer with.
public enum BroadcastLogConformanceFlagged {
    /// The entries a reader may rely on (`<sce:retention>`): the last `retention`
    /// published.
    public static let retention: Int = 3

    /// The slots the log keeps: one more than the retention, because the slot being
    /// overwritten is in flux and cannot be promised to a reader.
    public static let slots: Int = retention + 1

    /// The most bytes the element `broadcast_log_conformance_flagged_event` encodes to, read from its
    /// document when this file was generated.
    public static let maxEncodedBytes: Int = 3

    /// The words of an entry that hold what the element's codec asks its caller for
    /// (ADR 0062 section 4): the flag inputs, one bit each in the order of the
    /// codec's signature.
    public static let contextWords: Int = 1

    /// The 64-bit words an entry holds: one for the length of the encoded element,
    /// `contextWords` for its context, then as many as `maxEncodedBytes` bytes take at
    /// eight to a word, that is `1 + contextWords + ceil(maxEncodedBytes / 8)`.
    public static let words: Int = 1 + contextWords + (maxEncodedBytes + 7) / 8

    /// What the document requires of publish and read (`<sce:progress>`).
    public static let declaredProgress: String = "wait-free"

    /// An empty log of `retention` entries. Its storage is allocated here and never again.
    public static func makeLog() -> SceForgeRuntime.BroadcastLog {
        SceForgeRuntime.BroadcastLog(words: words, slots: slots)
    }

    /// The flag inputs the element's codec takes from its caller, which the log stores
    /// with each entry: what `publish` was given is what a reader gets back, and what
    /// the entry is decoded under.
    public struct Flags: Equatable, Sendable {
        public var hasValue: Bool

        public init(hasValue: Bool) {
            self.hasValue = hasValue
        }
    }

    /// What a typed publish did.
    public enum Publish: Equatable, Sendable {
        /// The element was stored as the entry `seq`.
        case published(seq: UInt64)

        /// The element's codec refused to write it, and nothing was published. An element
        /// that encodes to more than `maxEncodedBytes` bytes is refused with
        /// `CodecError.bufferOverflow` by the bounded sink: an entry is never a truncated
        /// element.
        case refused(CodecError)
    }

    /// What a typed read found: the runtime's three answers, with the entry decoded.
    public enum Read: Equatable, Sendable {
        /// The entry `seq`, decoded, with what it was published under; the reader now holds
        /// `seq + 1`.
        case item(seq: UInt64, element: BroadcastLogConformanceFlaggedEvent, flags: Flags)

        /// The reader is at the head: nothing newer is published.
        case empty

        /// The reader had fallen behind the window and missed `skipped` (at least one)
        /// entries; it now holds the oldest entry retained.
        case lagged(skipped: UInt64)

        /// The entry `seq` was copied whole, and is not a value this log wrote: its stored
        /// length is above `maxEncodedBytes`, or its bytes do not decode as a
        /// `BroadcastLogConformanceFlaggedEvent`. The reader has moved past it either way.
        case unreadable(seq: UInt64, why: String)
    }

    /// Publishes `element`: encodes it into a sink of `maxEncodedBytes` bytes, packs its
    /// length and its bytes (little-endian, eight to a word) into `words` words, and hands
    /// them to the runtime, which never refuses and never waits.
    ///
    /// An element that encodes to more than `maxEncodedBytes` bytes is refused by the codec
    /// and nothing is published (`Publish.refused`). Past the runtime's horizon of entries
    /// the runtime stops the program, which no run reaches in practice.
    ///
    /// `flags` are what the element's codec takes from its caller as flag inputs: they are
    /// encoded under here and stored in the entry's context words, so that every reader
    /// decodes the entry under what it was published under. The log does not check that the
    /// flags agree with `element`; the publisher gives what it knows, as every caller of this
    /// codec does.
    public static func publish(
        _ publisher: SceForgeRuntime.BroadcastLogPublisher, _ element: BroadcastLogConformanceFlaggedEvent, flags: Flags
    ) -> Publish {
        var sink = BoundedSink(capacity: maxEncodedBytes)
        do throws(CodecError) {
            try element.encode(into: &sink, has_value: flagInput(flags.hasValue))
        } catch {
            return .refused(error)
        }
        let bytes = sink.written
        var entry = [UInt64](repeating: 0, count: words)
        entry[0] = UInt64(bytes.count)
        if flags.hasValue {
            entry[1 + 0] |= UInt64(1) << UInt64(0)
        }
        for (index, byte) in bytes.enumerated() {
            entry[2 + index / 8] |= UInt64(byte) << UInt64(8 * (index % 8))
        }
        return .published(seq: publisher.publish(entry))
    }

    /// Reads the entry at `reader`'s cursor and decodes it. The runtime answers an item only
    /// after the copy it made was checked against the slot, which is what makes decoding it
    /// safe: a torn copy is a lag and is never decoded.
    ///
    /// A stored length above `maxEncodedBytes` is not a length this log wrote, and is
    /// `Read.unreadable` and never an index out of range.
    public static func read(
        _ log: SceForgeRuntime.BroadcastLog, _ reader: inout SceForgeRuntime.BroadcastLogReader
    ) -> Read {
        var entry = [UInt64](repeating: 0, count: words)
        switch log.read(&reader, into: &entry) {
        case .item(let seq):
            return decodeEntry(seq: seq, entry: entry)
        case .empty:
            return .empty
        case .lagged(let skipped):
            return .lagged(skipped: skipped)
        }
    }

    /// A flag input as the codec takes it: one bit, in a byte that is 1 or 0.
    private static func flagInput(_ on: Bool) -> UInt8 {
        on ? 1 : 0
    }

    /// The entry `seq`, whose words the runtime copied whole into `entry`.
    private static func decodeEntry(seq: UInt64, entry: [UInt64]) -> Read {
        let stored = entry[0]
        if stored > UInt64(maxEncodedBytes) {
            return .unreadable(
                seq: seq,
                why: "the stored length \(stored) is above the \(maxEncodedBytes) bytes this log holds")
        }
        var bytes = [UInt8](repeating: 0, count: Int(stored))
        for index in 0..<bytes.count {
            bytes[index] = UInt8(truncatingIfNeeded: entry[2 + index / 8] >> UInt64(8 * (index % 8)))
        }
        // What the entry was published under: the context words are written by `publish` alone
        // and a torn copy is never decoded, so a flag is a bit and the tag a byte.
        let flags = Flags(
            hasValue: ((entry[1 + 0] >> UInt64(0)) & 1) == 1
        )
        var cursor = SceCursor(bytes)
        do throws(CodecError) {
            let element = try BroadcastLogConformanceFlaggedEvent.decode(&cursor, has_value: flagInput(flags.hasValue))
            return .item(seq: seq, element: element, flags: flags)
        } catch {
            return .unreadable(
                seq: seq, why: "the stored bytes do not decode as a BroadcastLogConformanceFlaggedEvent: \(error)")
        }
    }
}
