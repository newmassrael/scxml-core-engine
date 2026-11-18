// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_ext_envelope.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohExtEnvelope` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohExtEnvelope.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohExtEnvelope`, which conforms itself.
public protocol CodecZenohExtEnvelopeSource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    var header_flags: UInt8 { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohExtEnvelope.ExtensionsBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohExtEnvelope: Equatable, Sendable, CodecZenohExtEnvelopeSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The field at byte 0, big-endian.
    public var header_flags: UInt8 = 0

    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohExtEnvelope.ExtensionsBound> = .fewest(of: CodecZenohExtEntry())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header_flags: UInt8, extensions: BoundedList<Extensions_Entry, CodecZenohExtEnvelope.ExtensionsBound>) {
        self.header_flags = header_flags
        self.extensions = extensions
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohExtEnvelope {
        var reader = cursor
        let header_flags = try reader.readInteger(UInt8.self, .big)
        var extensions_items: [CodecZenohExtEntry] = []
        var extensions_more = false
        while true {
            let entry = try CodecZenohExtEntry.decode(&reader)
            extensions_more = entry.Z
            extensions_items.append(entry)
            if !extensions_more { break }
        }
        let extensions = try BoundedList<CodecZenohExtEntry, CodecZenohExtEnvelope.ExtensionsBound>(decoding: extensions_items)
        cursor = reader
        return CodecZenohExtEnvelope(header_flags: header_flags, extensions: extensions)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohExtEnvelopeSource` is written,
    /// with no copy into `CodecZenohExtEnvelope`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohExtEnvelopeSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(source.header_flags, to: &frame, .big)
        var extensions_entries = BytesSink()
        var extensions_left = source.extensions.count
        for entry in source.extensions {
            extensions_left -= 1
            try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
        }
        frame += extensions_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
