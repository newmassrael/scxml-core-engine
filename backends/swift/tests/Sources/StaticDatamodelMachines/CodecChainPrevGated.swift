// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_chain_prev_gated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainPrevGated` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainPrevGated.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainPrevGated`, which conforms itself.
public protocol CodecChainPrevGatedSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainPrevEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainPrevEntrySource
    var head: UInt8 { get }
    var entries: Lawful<BoundedList<Entries_Entry, CodecChainPrevGated.EntriesBound>, CodecChainPrevGated.EntriesLaw<Entries_Entry>>? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainPrevGated: Equatable, Sendable, CodecChainPrevGatedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 4
    }

    /// For the law of `entries` (docs/adr/0065 §6 R8): what each entry says of `tagged`, read off any source of its codec.
    public enum EntriesTaggedSaid<Entry: CodecChainPrevEntrySource>: SceSaid {
        public static func says(_ entry: Entry) -> Bool? {
            CodecChainPrevEntry.inputTagged(entry) != 0
        }
    }

    /// For the law of `entries` (docs/adr/0065 §6 R8): what each entry says of `after_marker`, read off any source of its codec.
    public enum EntriesAfterMarkerSaid<Entry: CodecChainPrevEntrySource>: SceSaid {
        public static func says(_ entry: Entry) -> Bool? {
            CodecChainPrevEntry.inputAfterMarker(entry) != 0
        }
    }

    /// For the law of `entries` (docs/adr/0065 §6 R8): whether an entry is of the kind 0x4, after which the next says `after_marker` set, read off any source of its codec.
    public enum EntriesAfterMarkerKind<Entry: CodecChainPrevEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            (chainIdentifier(entry.header) & 0x7F) == 0x4
        }
    }

    /// The law the entries of `entries` say the inputs it hands them by (docs/adr/0065 §6 R8),
    /// over any source of their codec: its type holds every entry to it where a value of it is made.
    public typealias EntriesLaw<Entry: CodecChainPrevEntrySource> = SceLaws<SceUniform<EntriesTaggedSaid<Entry>>, SceAfterKey<EntriesAfterMarkerSaid<Entry>, EntriesAfterMarkerKind<Entry>>>

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecChainPrevEntry

    /// The field at byte 0, big-endian.
    public var head: UInt8 = 0

    /// Bit 0 of `head` (flag `T`).
    public var T: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// Bit 1 of `head` (flag `E`).
    public var E: Bool {
        get { (head & 0x2) != 0 }
        set { head = newValue ? (head | 0x2) : (head & ~0x2) }
    }

    /// On the wire only when `(head & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecChainPrevEntry`, one after another, at most 4: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var entries: Lawful<BoundedList<CodecChainPrevEntry, CodecChainPrevGated.EntriesBound>, CodecChainPrevGated.EntriesLaw<CodecChainPrevEntry>>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, entries: Lawful<BoundedList<Entries_Entry, CodecChainPrevGated.EntriesBound>, CodecChainPrevGated.EntriesLaw<Entries_Entry>>?) {
        self.head = head
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainPrevGated {
        var reader = cursor
        let head = try reader.readInteger(UInt8.self, .big)
        var entries: Lawful<BoundedList<CodecChainPrevEntry, CodecChainPrevGated.EntriesBound>, CodecChainPrevGated.EntriesLaw<CodecChainPrevEntry>>? = nil
        if (head & 0x2) != 0 {
            var _prev_entries_after_marker: UInt8 = 0
            var entries_items: [CodecChainPrevEntry] = []
            var entries_more = false
            for _ in 0..<4 {
                let entry = try CodecChainPrevEntry.decode(&reader, after_marker: _prev_entries_after_marker, tagged: UInt8(truncatingIfNeeded: (head >> 0) & 0x1))
                entries_more = entry.Z
                entries_items.append(entry)
                _prev_entries_after_marker = ((chainIdentifier(entry.header) & 127) == 4) ? 1 : 0
                if !entries_more { break }
            }
            if entries_more && reader.remaining == 0 {
                throw CodecError.needMoreBytes
            }
            if entries_more {
                throw CodecError.tooManyElements
            }
            entries = try Lawful<BoundedList<CodecChainPrevEntry, CodecChainPrevGated.EntriesBound>, CodecChainPrevGated.EntriesLaw<CodecChainPrevEntry>>(decoding: try BoundedList<CodecChainPrevEntry, CodecChainPrevGated.EntriesBound>(decoding: entries_items), start: SceStarts(((head >> 0) & 1) != 0, SceFixedStart()))
        }
        cursor = reader
        return CodecChainPrevGated(head: head, entries: entries)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainPrevGatedSource` is written,
    /// with no copy into `CodecChainPrevGated`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainPrevGatedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var head_folded = source.head
        if source.entries != nil { head_folded = (head_folded & ~0x1) | ((UInt8(truncatingIfNeeded: (source.entries.map { $0.start.first ? 1 : 0 } ?? 0)) & 0x1) << 0) }
        head_folded = (head_folded & ~0x2) | ((source.entries != nil) ? 0x2 : 0)
        var frame: [UInt8] = []
        appendInteger(head_folded, to: &frame, .big)
        if let held = source.entries {
            var entries_entries = BytesSink()
            var entries_left = held.count
            for entry in held {
                entries_left -= 1
                try CodecChainPrevEntry.encodeSource(entry, into: &entries_entries, overrides: CodecChainPrevEntry.Overrides(headerZ: entries_left != 0))
            }
            frame += entries_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
