// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_query.scxml:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohQuery` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohQuery.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohQuery`, which conforms itself.
public protocol CodecZenohQuerySource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    var header: UInt8 { get }
    var consolidation: UInt8? { get }
    var parameters: BoundedBytes<CodecZenohQuery.ParametersBound>? { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohQuery.ExtensionsBound>? { get }
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohQuery: Equatable, Sendable, CodecZenohQuerySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `parameters` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ParametersBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = nil
    }

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = nil
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x3

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `C`).
    public var C: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `P`).
    public var P: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var consolidation: UInt8? = nil

    /// On the wire only when `(header & 0x40) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `parameters_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var parameters: BoundedBytes<CodecZenohQuery.ParametersBound>? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: to the end of the frame. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohQuery.ExtensionsBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, consolidation: UInt8?, parameters: BoundedBytes<CodecZenohQuery.ParametersBound>?, extensions: BoundedList<Extensions_Entry, CodecZenohQuery.ExtensionsBound>?) {
        self.header = header
        self.consolidation = consolidation
        self.parameters = parameters
        self.extensions = extensions
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohQuery {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var consolidation: UInt8? = nil
        if (header & 0x20) != 0 {
            consolidation = try reader.readInteger(UInt8.self, .big)
        }
        var parameters_len: UInt64? = nil
        if (header & 0x40) != 0 {
            parameters_len = try reader.readVle(UInt64.self)
        }
        var parameters: BoundedBytes<CodecZenohQuery.ParametersBound>? = nil
        if (header & 0x40) != 0 {
            parameters = try BoundedBytes<CodecZenohQuery.ParametersBound>(decoding: Array(try reader.readSlice(Int(clamping: (parameters_len ?? 0)))))
        }
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohQuery.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            while reader.remaining > 0 {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_items.append(entry)
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohQuery.ExtensionsBound>(decoding: extensions_items)
        }
        cursor = reader
        return CodecZenohQuery(header: header, consolidation: consolidation, parameters: parameters, extensions: extensions)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohQuerySource` is written,
    /// with no copy into `CodecZenohQuery`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohQuerySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let parameters_len_derived: UInt64 = source.parameters != nil ? UInt64((source.parameters?.byteCount ?? 0)) : 0
        var header_folded = source.header
        header_folded = (header_folded & ~0x20) | ((source.consolidation != nil) ? 0x20 : 0)
        header_folded = (header_folded & ~0x40) | ((source.parameters != nil) ? 0x40 : 0)
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = source.consolidation {
            appendInteger(held, to: &frame, .big)
        }
        if ((header_folded >> 6) & 1) != 0 {
            appendVle(parameters_len_derived, to: &frame)
        }
        if let held = source.parameters {
            held.append(to: &frame)
        }
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            for entry in held {
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries)
            }
            frame += extensions_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
