// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d95af6fbc211ea1c5b1e249d16b701ebad41657a9f4b213349f1c668ce4ccbb
// SCE-MAP: broadcast_log_conformance_chosen.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="broadcast-log")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// ADR 0062 broadcast-log — a bounded log that one execution context appends
// to and any number of readers read, each by a cursor of its own. The document
// states the contract (element type `broadcast_log_conformance_choice`, one producer,
// `wait-free`, the last 3 entries retained) and names no algorithm;
// the algorithm is the runtime's sequence-locked ring.
//
// An entry is the element's wire bytes, never a value: the log stores the
// bytes the element document's codec writes, so a copy that was torn is thrown
// away unread, and the element a reader gets is a value of its own, decoded
// from a buffer only that read holds.
//
// Operations (from `SceForgeRuntime`, typed here over the element):
//   BroadcastLogConformanceChosen.publish(publisher, element) -> BroadcastLogConformanceChosen.Publish
//       wait-free; the entry's sequence number, or why the element was refused
//   BroadcastLogConformanceChosen.read(log, &reader) -> BroadcastLogConformanceChosen.Read
//       wait-free; an item, nothing new, how many were missed, or an entry that does not decode
// A reader is `log.readerOldest()`, `log.readerHead()`, `log.readerAt(n)` or
// `log.readerDeferred()` and owns its cursor; the publisher is `log.publisher()`,
// taken once and given back by `release()` or by dropping it.
//
// The element's codec asks its caller for a variant tag. Those are
// part of the entry: the log stores them in the entry's context words beside the
// bytes, so every reader decodes an entry under what it was published under (ADR 0062
// section 4).
// The tag is not given: it is a function of the element, which `publish` reads off it
// (`element.tag`) and stores, and the tag of an `item`'s element is its own.
//
// The names are the members of the enum `BroadcastLogConformanceChosen` and not top-level
// declarations: every unit of a SwiftPM target shares one module, so the
// constants of two logs would otherwise be one redeclared name.

import SceForgeRuntime

/// The log of `broadcast_log_conformance_choice` entries: its constants, its typed `publish` and
/// `read`, and the results they answer with.
public enum BroadcastLogConformanceChosen {
    /// The entries a reader may rely on (`<sce:retention>`): the last `retention`
    /// published.
    public static let retention: Int = 3

    /// The slots the log keeps: one more than the retention, because the slot being
    /// overwritten is in flux and cannot be promised to a reader.
    public static let slots: Int = retention + 1

    /// The most bytes the element `broadcast_log_conformance_choice` encodes to, read from its
    /// document when this file was generated.
    public static let maxEncodedBytes: Int = 2

    /// The words of an entry that hold what the element's codec asks its caller for
    /// (ADR 0062 section 4): one word that holds the variant's tag.
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
        /// The tag it was decoded under is the element's own (`element.tag`).
        case item(seq: UInt64, element: BroadcastLogConformanceChoice)

        /// The reader is at the head: nothing newer is published.
        case empty

        /// The reader had fallen behind the window and missed `skipped` (at least one)
        /// entries; it now holds the oldest entry retained.
        case lagged(skipped: UInt64)

        /// The entry `seq` was copied whole, and is not a value this log wrote: its stored
        /// length is above `maxEncodedBytes`, or its bytes do not decode as a
        /// `BroadcastLogConformanceChoice`. The reader has moved past it either way.
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
    /// The variant tag the element's decode takes is not a parameter: it is a function of
    /// `element`, `element.tag`, the tag of the arm it holds. It is stored in the entry's
    /// context words, so that no entry is stored under a tag its bytes disagree with and
    /// every reader decodes the entry under the arm it was published as.
    public static func publish(
        _ publisher: SceForgeRuntime.BroadcastLogPublisher, _ element: BroadcastLogConformanceChoice
    ) -> Publish {
        var sink = BoundedSink(capacity: maxEncodedBytes)
        do throws(CodecError) {
            try element.encode(into: &sink)
        } catch {
            return .refused(error)
        }
        let bytes = sink.written
        var entry = [UInt64](repeating: 0, count: words)
        entry[0] = UInt64(bytes.count)
        entry[1 + 0] = UInt64(element.tag)
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
        let tag = UInt8(truncatingIfNeeded: entry[1 + 0])
        var cursor = SceCursor(bytes)
        do throws(CodecError) {
            let element = try BroadcastLogConformanceChoice.decode(&cursor, tag: tag)
            return .item(seq: seq, element: element)
        } catch {
            return .unreadable(
                seq: seq, why: "the stored bytes do not decode as a BroadcastLogConformanceChoice: \(error)")
        }
    }
}
