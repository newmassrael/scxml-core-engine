// SCE-GENERATED — DO NOT EDIT
// source-hash: 1bec5ed2a01f3cd93c6ef8e1f47acf412d092e79a253fb996d7e4effc9c20be8
// source: codec_chain_flag_forward.scxml
// SCE-MAP: codec_chain_flag_forward.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainFlagForward` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainFlagForward.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainFlagForward`, which conforms itself.
public protocol CodecChainFlagForwardSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainFlagEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainFlagEntrySource
    var head: UInt8 { get }
    var entries: Lawful<BoundedList<Entries_Entry, CodecChainFlagForward.EntriesBound>, CodecChainFlagForward.EntriesLaw<Entries_Entry>>? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainFlagForward: Equatable, Sendable, CodecChainFlagForwardSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 1
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

    /// Bit 0 of `head` (flag `E`).
    public var E: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// On the wire only when `(head & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecChainFlagEntry`, one after another, at most 4: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var entries: Lawful<BoundedList<CodecChainFlagEntry, CodecChainFlagForward.EntriesBound>, CodecChainFlagForward.EntriesLaw<CodecChainFlagEntry>>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, entries: Lawful<BoundedList<Entries_Entry, CodecChainFlagForward.EntriesBound>, CodecChainFlagForward.EntriesLaw<Entries_Entry>>?) {
        self.head = head
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, wide: UInt8) throws(CodecError) -> CodecChainFlagForward {
        var reader = cursor
        let head = try reader.readInteger(UInt8.self, .big)
        var entries: Lawful<BoundedList<CodecChainFlagEntry, CodecChainFlagForward.EntriesBound>, CodecChainFlagForward.EntriesLaw<CodecChainFlagEntry>>? = nil
        if (head & 0x1) != 0 {
            var entries_items: [CodecChainFlagEntry] = []
            var entries_more = false
            for _ in 0..<4 {
                let entry = try CodecChainFlagEntry.decode(&reader, wide: wide)
                entries_more = entry.Z
                entries_items.append(entry)
                if !entries_more { break }
            }
            if entries_more && reader.remaining == 0 {
                throw CodecError.needMoreBytes
            }
            if entries_more {
                throw CodecError.tooManyElements
            }
            entries = try Lawful<BoundedList<CodecChainFlagEntry, CodecChainFlagForward.EntriesBound>, CodecChainFlagForward.EntriesLaw<CodecChainFlagEntry>>(decoding: try BoundedList<CodecChainFlagEntry, CodecChainFlagForward.EntriesBound>(decoding: entries_items), start: (wide & 0x01) != 0)
        }
        cursor = reader
        return CodecChainFlagForward(head: head, entries: entries)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainFlagForwardSource` is written,
    /// with no copy into `CodecChainFlagForward`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainFlagForwardSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var head_folded = source.head
        head_folded = (head_folded & ~0x1) | ((source.entries != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(head_folded, to: &frame, .big)
        if let held = source.entries {
            var entries_entries = BytesSink()
            var entries_left = held.count
            for entry in held {
                entries_left -= 1
                try CodecChainFlagEntry.encodeSource(entry, into: &entries_entries, overrides: CodecChainFlagEntry.Overrides(headerZ: entries_left != 0))
            }
            frame += entries_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `wide` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0, and `nil` where the value says nothing of it (§6 R4e). A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputWide<Source: CodecChainFlagForwardSource>(_ source: Source) -> UInt8? {
        source.entries.map { $0.start ? 1 : 0 }
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
