// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a95571dea1fd85de0733bf9fbb3ce212e8f945dd72643c54cf5129c6db19502
// source: codec_chain_has_tagged.scxml
// SCE-MAP: codec_chain_has_tagged.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecChainHasTagged` the gates `entries.has(0x7) || entries.has(0x9) || header.wide` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries_Entry: CodecChainHasTaggedEntrySource> {
    /// While entries.has(0x7) set, entries.has(0x9) set, header.wide set.
    case has0x7Has0x9Wide(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>, SceNoKinds<Entries_Entry>>, priority: UInt8)
    /// While entries.has(0x7) set, entries.has(0x9) set, header.wide clear.
    case has0x7Has0x9(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>, SceNoKinds<Entries_Entry>>, priority: UInt8)
    /// While entries.has(0x7) set, entries.has(0x9) clear, header.wide set.
    case has0x7Wide(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceNoKinds<Entries_Entry>>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>, priority: UInt8, checksum: UInt16)
    /// While entries.has(0x7) clear, entries.has(0x9) set, header.wide set.
    case has0x9Wide(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceNoKinds<Entries_Entry>>>, priority: UInt8)
    /// While entries.has(0x7) set, entries.has(0x9) clear, header.wide clear.
    case has0x7(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceNoKinds<Entries_Entry>>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>, priority: UInt8, checksum: UInt16)
    /// While entries.has(0x7) clear, entries.has(0x9) set, header.wide clear.
    case has0x9(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceNoKinds<Entries_Entry>>>)
    /// While entries.has(0x7) clear, entries.has(0x9) clear, header.wide set.
    case wide(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceNoKinds<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>>, priority: UInt8, checksum: UInt16)
    /// While entries.has(0x7) clear, entries.has(0x9) clear, header.wide clear.
    case neither(entries: SceChain<Entries_Entry, CodecChainHasTagged.EntriesBound, SceNoKinds<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind7<Entries_Entry>, SceKindsOf<CodecChainHasTagged.EntriesKind9<Entries_Entry>, SceNoKinds<Entries_Entry>>>>, checksum: UInt16)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .has0x7Has0x9Wide: return 0
        case .has0x7Has0x9: return 1
        case .has0x7Wide: return 2
        case .has0x9Wide: return 3
        case .has0x7: return 4
        case .has0x9: return 5
        case .wide: return 6
        case .neither: return 7
        }
    }
}

extension CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide: Equatable where Entries_Entry: Equatable {}

extension CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide: Sendable where Entries_Entry: Sendable {}

/// The elements a value of `CodecChainHasTagged` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasTagged.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasTagged`, which conforms itself.
public protocol CodecChainHasTaggedSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainHasTaggedEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainHasTaggedEntrySource
    var header: UInt8 { get }
    var entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries_Entry> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasTagged: Equatable, Sendable, CodecChainHasTaggedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 3
    }

    /// Whether an entry of `entries` is of the kind 0x7 a gate asks the chain for (docs/adr/0065
    /// §6 R6), read off any source of its codec: what the arms of the chain's kinds are held to.
    public enum EntriesKind7<Entry: CodecChainHasTaggedEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            chainIdentifier(entry.entry_type) == 0x7
        }
    }

    /// Whether an entry of `entries` is of the kind 0x9 a gate asks the chain for (docs/adr/0065
    /// §6 R6), read off any source of its codec: what the arms of the chain's kinds are held to.
    public enum EntriesKind9<Entry: CodecChainHasTaggedEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            chainIdentifier(entry.entry_type) == 0x9
        }
    }

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecChainHasTaggedEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 0 of `header` (flag `wide`).
    public var wide: Bool {
        get { (header & 0x1) != 0 }
        set { header = newValue ? (header | 0x1) : (header & ~0x1) }
    }

    /// The elements the gates `entries.has(0x7) || entries.has(0x9) || header.wide` decide together.
    public var entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries_Entry> = .neither(entries: .init(defaultOf: .fewest(of: CodecChainHasTaggedEntry())), checksum: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries_Entry>) {
        self.header = header
        self.entries_has_0x7_has_0x9_header_wide = entries_has_0x7_has_0x9_header_wide
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasTagged {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var entries_items: [CodecChainHasTaggedEntry] = []
        var entries_more = false
        for _ in 0..<3 {
            let entry = try CodecChainHasTaggedEntry.decode(&reader)
            entries_more = entry.more
            entries_items.append(entry)
            if !entries_more { break }
        }
        if entries_more && reader.remaining == 0 {
            throw CodecError.needMoreBytes
        }
        if entries_more {
            throw CodecError.tooManyElements
        }
        let entries = try BoundedList<CodecChainHasTaggedEntry, CodecChainHasTagged.EntriesBound>(decoding: entries_items)
        let _has_entries_7 = entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })
        let _has_entries_9 = entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 9 })
        var priority: UInt8? = nil
        if _has_entries_7 || (header & 0x1) != 0 {
            priority = try reader.readInteger(UInt8.self, .big)
        }
        var checksum: UInt16? = nil
        if !_has_entries_9 {
            checksum = try reader.readInteger(UInt16.self, .big)
        }
        let entries_has_0x7_has_0x9_header_wide_group: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide<Entries_Entry>
        if _has_entries_7 && _has_entries_9 && ((header >> 0) & 1) != 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x7Has0x9Wide(entries: try SceChain(decoding: entries), priority: priority!)
        } else if _has_entries_7 && _has_entries_9 && ((header >> 0) & 1) == 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x7Has0x9(entries: try SceChain(decoding: entries), priority: priority!)
        } else if _has_entries_7 && !_has_entries_9 && ((header >> 0) & 1) != 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x7Wide(entries: try SceChain(decoding: entries), priority: priority!, checksum: checksum!)
        } else if !_has_entries_7 && _has_entries_9 && ((header >> 0) & 1) != 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x9Wide(entries: try SceChain(decoding: entries), priority: priority!)
        } else if _has_entries_7 && !_has_entries_9 && ((header >> 0) & 1) == 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x7(entries: try SceChain(decoding: entries), priority: priority!, checksum: checksum!)
        } else if !_has_entries_7 && _has_entries_9 && ((header >> 0) & 1) == 0 {
            entries_has_0x7_has_0x9_header_wide_group = .has0x9(entries: try SceChain(decoding: entries))
        } else if !_has_entries_7 && !_has_entries_9 && ((header >> 0) & 1) != 0 {
            entries_has_0x7_has_0x9_header_wide_group = .wide(entries: try SceChain(decoding: entries), priority: priority!, checksum: checksum!)
        } else {
            entries_has_0x7_has_0x9_header_wide_group = .neither(entries: try SceChain(decoding: entries), checksum: checksum!)
        }
        cursor = reader
        return CodecChainHasTagged(header: header, entries_has_0x7_has_0x9_header_wide: entries_has_0x7_has_0x9_header_wide_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasTaggedSource` is written,
    /// with no copy into `CodecChainHasTagged`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasTaggedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var entries_member: [Source.Entries_Entry]? = nil
        var priority_member: UInt8? = nil
        var checksum_member: UInt16? = nil
        switch source.entries_has_0x7_has_0x9_header_wide {
        case .has0x7Has0x9Wide(let entries, let priority): entries_member = entries.entries; priority_member = priority
        case .has0x7Has0x9(let entries, let priority): entries_member = entries.entries; priority_member = priority
        case .has0x7Wide(let entries, let priority, let checksum): entries_member = entries.entries; priority_member = priority; checksum_member = checksum
        case .has0x9Wide(let entries, let priority): entries_member = entries.entries; priority_member = priority
        case .has0x7(let entries, let priority, let checksum): entries_member = entries.entries; priority_member = priority; checksum_member = checksum
        case .has0x9(let entries): entries_member = entries.entries
        case .wide(let entries, let priority, let checksum): entries_member = entries.entries; priority_member = priority; checksum_member = checksum
        case .neither(let entries, let checksum): entries_member = entries.entries; checksum_member = checksum
        }
        var header_folded = source.header
        header_folded = (header_folded & ~0x1) | ([0, 2, 3, 6].contains(source.entries_has_0x7_has_0x9_header_wide.arm) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = entries_member {
            var entries_entries = BytesSink()
            var entries_left = held.count
            for entry in held {
                entries_left -= 1
                try CodecChainHasTaggedEntry.encodeSource(entry, into: &entries_entries, overrides: CodecChainHasTaggedEntry.Overrides(ctlMore: entries_left != 0))
            }
            frame += entries_entries.bytes
        }
        if let held = priority_member {
            appendInteger(held, to: &frame, .big)
        }
        if let held = checksum_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
