// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_chain_has_input.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHasInput` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasInput.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasInput`, which conforms itself.
public protocol CodecChainHasInputSource {
    /// What each entry of `entries` is presented as: a source of `CodecChainHasTaggedEntry`, written by its encode.
    associatedtype Entries_Entry: CodecChainHasTaggedEntrySource
    var entries: BoundedList<Entries_Entry, CodecChainHasInput.EntriesBound> { get }
    var priority: UInt8? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasInput: Equatable, Sendable, CodecChainHasInputSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 3
    }

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecChainHasTaggedEntry

    /// Entries of `CodecChainHasTaggedEntry`, one after another, at most 3: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var entries: BoundedList<CodecChainHasTaggedEntry, CodecChainHasInput.EntriesBound> = .fewest(of: CodecChainHasTaggedEntry())

    /// On the wire only when `_has_entries_7 || (urgent & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var priority: UInt8? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(entries: BoundedList<Entries_Entry, CodecChainHasInput.EntriesBound>, priority: UInt8?) {
        self.entries = entries
        self.priority = priority
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, urgent: UInt8) throws(CodecError) -> CodecChainHasInput {
        var reader = cursor
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
        let entries = try BoundedList<CodecChainHasTaggedEntry, CodecChainHasInput.EntriesBound>(decoding: entries_items)
        let _has_entries_7 = entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })
        var priority: UInt8? = nil
        if _has_entries_7 || (urgent & 0x1) != 0 {
            priority = try reader.readInteger(UInt8.self, .big)
        }
        cursor = reader
        return CodecChainHasInput(entries: entries, priority: priority)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasInputSource` is written,
    /// with no copy into `CodecChainHasInput`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasInputSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, urgent: UInt8) throws(CodecError) {
        if (source.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 0x7 }) || urgent != 0) != (source.priority != nil) { throw CodecError.presentIfMismatch }
        var frame: [UInt8] = []
        var entries_entries = BytesSink()
        var entries_left = source.entries.count
        for entry in source.entries {
            entries_left -= 1
            try CodecChainHasTaggedEntry.encodeSource(entry, into: &entries_entries, overrides: CodecChainHasTaggedEntry.Overrides(ctlMore: entries_left != 0))
        }
        frame += entries_entries.bytes
        if let held = source.priority {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink, urgent: UInt8) throws(CodecError) {
        try Self.encodeSource(self, into: &sink, urgent: urgent)
    }
}
