// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_repeat_flag_forward_group.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecRepeatFlagForwardGroup` the gates `head.L` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecRepeatFlagForwardGroupHeadL<Entries_Entry: CodecChainFlagEntrySource> {
    /// While head.L set.
    case l(entries: Lawful<BoundedList<Entries_Entry, CodecRepeatFlagForwardGroup.EntriesBound>, CodecRepeatFlagForwardGroup.EntriesLaw<Entries_Entry>>)    /// While head.L clear.
    case notL(note: UInt8)
    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .l: return 0
        case .notL: return 1
        }
    }
}

extension CodecRepeatFlagForwardGroupHeadL: Equatable where Entries_Entry: Equatable {}

extension CodecRepeatFlagForwardGroupHeadL: Sendable where Entries_Entry: Sendable {}

/// The elements a value of `CodecRepeatFlagForwardGroup` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecRepeatFlagForwardGroup.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecRepeatFlagForwardGroup`, which conforms itself.
public protocol CodecRepeatFlagForwardGroupSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainFlagEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainFlagEntrySource
    var head: UInt8 { get }
    var head_l: CodecRepeatFlagForwardGroupHeadL<Entries_Entry> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatFlagForwardGroup: Equatable, Sendable, CodecRepeatFlagForwardGroupSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

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

    /// Bit 0 of `head` (flag `L`).
    public var L: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// The elements the gates `head.L` decide together.
    public var head_l: CodecRepeatFlagForwardGroupHeadL<Entries_Entry> = .notL(note: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, head_l: CodecRepeatFlagForwardGroupHeadL<Entries_Entry>) {
        self.head = head
        self.head_l = head_l
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, wide: UInt8) throws(CodecError) -> CodecRepeatFlagForwardGroup {
        var reader = cursor
        let head = try reader.readInteger(UInt8.self, .big)
        var count: UInt8? = nil
        if (head & 0x1) != 0 {
            count = try reader.readInteger(UInt8.self, .big)
        }
        var entries: Lawful<BoundedList<CodecChainFlagEntry, CodecRepeatFlagForwardGroup.EntriesBound>, CodecRepeatFlagForwardGroup.EntriesLaw<CodecChainFlagEntry>>? = nil
        if (head & 0x1) != 0 {
            if (count ?? 0) > 4 {
                throw CodecError.tooManyElements
            }
            var entries_items: [CodecChainFlagEntry] = []
            for _ in 0..<Int(clamping: (count ?? 0)) {
                entries_items.append(try CodecChainFlagEntry.decode(&reader, wide: wide))
            }
            entries = try Lawful<BoundedList<CodecChainFlagEntry, CodecRepeatFlagForwardGroup.EntriesBound>, CodecRepeatFlagForwardGroup.EntriesLaw<CodecChainFlagEntry>>(decoding: try BoundedList<CodecChainFlagEntry, CodecRepeatFlagForwardGroup.EntriesBound>(decoding: entries_items), start: (wide & 0x01) != 0)
        }
        var note: UInt8? = nil
        if (head & 0x1) == 0 {
            note = try reader.readInteger(UInt8.self, .big)
        }
        let head_l_group: CodecRepeatFlagForwardGroupHeadL<Entries_Entry>
        if ((head >> 0) & 1) != 0 {
            head_l_group = .l(entries: entries!)
        } else {
            head_l_group = .notL(note: note!)
        }
        cursor = reader
        return CodecRepeatFlagForwardGroup(head: head, head_l: head_l_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecRepeatFlagForwardGroupSource` is written,
    /// with no copy into `CodecRepeatFlagForwardGroup`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecRepeatFlagForwardGroupSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var entries_member: Lawful<BoundedList<Source.Entries_Entry, CodecRepeatFlagForwardGroup.EntriesBound>, CodecRepeatFlagForwardGroup.EntriesLaw<Source.Entries_Entry>>? = nil
        var note_member: UInt8? = nil
        switch source.head_l {
        case .l(let entries): entries_member = entries
        case .notL(let note): note_member = note
        }
        let count_derived: UInt8 = entries_member != nil ? UInt8((entries_member?.count ?? 0)) : 0
        var head_folded = source.head
        head_folded = (head_folded & ~0x1) | ((entries_member != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(head_folded, to: &frame, .big)
        if ((head_folded >> 0) & 1) != 0 {
            appendInteger(count_derived, to: &frame, .big)
        }
        if let held = entries_member {
            var entries_entries = BytesSink()
            for entry in held {
                try CodecChainFlagEntry.encodeSource(entry, into: &entries_entries)
            }
            frame += entries_entries.bytes
        }
        if let held = note_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `wide` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0, and `nil` where the value says nothing of it (§6 R4e). A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputWide<Source: CodecRepeatFlagForwardGroupSource>(_ source: Source) -> UInt8? {
        switch source.head_l {
        case .l(let held): return held.start ? 1 : 0
        case .notL: return nil
        }
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
