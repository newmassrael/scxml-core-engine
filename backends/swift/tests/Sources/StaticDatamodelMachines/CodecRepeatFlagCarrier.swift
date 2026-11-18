// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_repeat_flag_carrier.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecRepeatFlagCarrier` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecRepeatFlagCarrier.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecRepeatFlagCarrier`, which conforms itself.
public protocol CodecRepeatFlagCarrierSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainFlagEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainFlagEntrySource
    var head: UInt8 { get }
    var entries: Lawful<BoundedList<Entries_Entry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<Entries_Entry>> { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatFlagCarrier: Equatable, Sendable, CodecRepeatFlagCarrierSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 4
    }

    /// For the law of `entries` (docs/adr/0065 §6 R8): what each entry says of `wide`, read off any source of its codec.
    public enum EntriesWideSaid<Entry: CodecChainFlagEntrySource>: SceSaid {
        public static func says(_ entry: Entry) -> Bool? {
            CodecChainFlagEntry.inputWide(entry) != 0
        }
    }

    /// The law the entries of `entries` say the inputs it hands them by (docs/adr/0065 §6 R8),
    /// over any source of their codec: its type holds every entry to it where a value of it is made.
    public typealias EntriesLaw<Entry: CodecChainFlagEntrySource> = SceUniform<EntriesWideSaid<Entry>>

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecChainFlagEntry

    /// The field at byte 0, big-endian.
    public var head: UInt8 = 0

    /// Bit 0 of `head` (flag `W`).
    public var W: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// As many entries as `count` says, each a `CodecChainFlagEntry`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var entries: Lawful<BoundedList<CodecChainFlagEntry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<CodecChainFlagEntry>> = .init(defaultOf: .fewest(of: CodecChainFlagEntry()))

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, entries: Lawful<BoundedList<Entries_Entry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<Entries_Entry>>) {
        self.head = head
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatFlagCarrier {
        var reader = cursor
        let head = try reader.readInteger(UInt8.self, .big)
        let count = try reader.readInteger(UInt8.self, .big)
        if count > 4 {
            throw CodecError.tooManyElements
        }
        var entries_items: [CodecChainFlagEntry] = []
        for _ in 0..<Int(clamping: count) {
            entries_items.append(try CodecChainFlagEntry.decode(&reader, wide: UInt8(truncatingIfNeeded: (head >> 0) & 0x1)))
        }
        let entries = try Lawful<BoundedList<CodecChainFlagEntry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<CodecChainFlagEntry>>(decoding: try BoundedList<CodecChainFlagEntry, CodecRepeatFlagCarrier.EntriesBound>(decoding: entries_items), start: ((head >> 0) & 1) != 0)
        cursor = reader
        return CodecRepeatFlagCarrier(head: head, entries: entries)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecRepeatFlagCarrierSource` is written,
    /// with no copy into `CodecRepeatFlagCarrier`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecRepeatFlagCarrierSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var head_folded = source.head
        head_folded = (head_folded & ~0x1) | ((UInt8(truncatingIfNeeded: (source.entries.start ? 1 : 0)) & 0x1) << 0)
        let count_derived: UInt8 = UInt8(source.entries.count)
        var frame: [UInt8] = []
        appendInteger(head_folded, to: &frame, .big)
        appendInteger(count_derived, to: &frame, .big)
        var entries_entries = BytesSink()
        for entry in source.entries {
            try CodecChainFlagEntry.encodeSource(entry, into: &entries_entries)
        }
        frame += entries_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
