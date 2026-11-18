// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_chain_handed.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHanded` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHanded.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHanded`, which conforms itself.
public protocol CodecChainHandedSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainHandedEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainHandedEntrySource
    var entries: BoundedList<Entries_Entry, CodecChainHanded.EntriesBound> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHanded: Equatable, Sendable, CodecChainHandedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 4
    }

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecChainHandedEntry

    /// Entries of `CodecChainHandedEntry`, one after another, at most 4: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var entries: BoundedList<CodecChainHandedEntry, CodecChainHanded.EntriesBound> = .fewest(of: CodecChainHandedEntry())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(entries: BoundedList<Entries_Entry, CodecChainHanded.EntriesBound>) {
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHanded {
        var reader = cursor
        var _prev_entries_after_marker: UInt8 = 0
        var entries_items: [CodecChainHandedEntry] = []
        var entries_more = false
        for _ in 0..<4 {
            let entry = try CodecChainHandedEntry.decode(&reader, after_marker: _prev_entries_after_marker)
            entries_more = entry.Z
            entries_items.append(entry)
            _prev_entries_after_marker = (chainIdentifier(entry.extId) == 4) ? 1 : 0
            if !entries_more { break }
        }
        if entries_more && reader.remaining == 0 {
            throw CodecError.needMoreBytes
        }
        if entries_more {
            throw CodecError.tooManyElements
        }
        let entries = try BoundedList<CodecChainHandedEntry, CodecChainHanded.EntriesBound>(decoding: entries_items)
        cursor = reader
        return CodecChainHanded(entries: entries)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHandedSource` is written,
    /// with no copy into `CodecChainHanded`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHandedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var entries_entries = BytesSink()
        var entries_after_marker_previous: UInt8 = 0
        var entries_left = source.entries.count
        for entry in source.entries {
            entries_left -= 1
            try CodecChainHandedEntry.encodeSource(entry, into: &entries_entries, after_marker: entries_after_marker_previous, overrides: CodecChainHandedEntry.Overrides(headerZ: entries_left != 0))
            entries_after_marker_previous = ((chainIdentifier(entry.header) >> 0) & 0xF) == 0x4 ? 1 : 0
        }
        frame += entries_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
