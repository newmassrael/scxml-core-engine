// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_encoding.scxml:69 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohEncoding` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohEncoding.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohEncoding`, which conforms itself.
public protocol CodecZenohEncodingSource {
    var packed_id: UInt32 { get }
    var schema: BoundedText<CodecZenohEncoding.SchemaBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohEncoding: Equatable, Sendable, CodecZenohEncodingSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `schema` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum SchemaBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 255
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var packed_id: UInt32 = 0

    /// Bit 0 of `packed_id` (flag `has_schema`).
    public var hasSchema: Bool {
        get { (packed_id & 0x1) != 0 }
        set { packed_id = newValue ? (packed_id | 0x1) : (packed_id & ~0x1) }
    }

    /// On the wire only when `(packed_id & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `schema_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var schema: BoundedText<CodecZenohEncoding.SchemaBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(packed_id: UInt32, schema: BoundedText<CodecZenohEncoding.SchemaBound>?) {
        self.packed_id = packed_id
        self.schema = schema
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohEncoding {
        var reader = cursor
        let packed_id = try reader.readVle(UInt32.self)
        var schema_len: UInt64? = nil
        if (packed_id & 0x1) != 0 {
            schema_len = try reader.readVle(UInt64.self)
        }
        var schema: BoundedText<CodecZenohEncoding.SchemaBound>? = nil
        if (packed_id & 0x1) != 0 {
            if (schema_len ?? 0) > 255 {
                throw CodecError.lengthOutOfRange
            }
            schema = try BoundedText<CodecZenohEncoding.SchemaBound>(decoding: try reader.readString(Int(clamping: (schema_len ?? 0))))
        }
        cursor = reader
        return CodecZenohEncoding(packed_id: packed_id, schema: schema)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohEncodingSource` is written,
    /// with no copy into `CodecZenohEncoding`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohEncodingSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let schema_len_derived: UInt64 = source.schema != nil ? UInt64((source.schema?.byteCount ?? 0)) : 0
        var packed_id_folded = source.packed_id
        packed_id_folded = (packed_id_folded & ~0x1) | ((source.schema != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendVle(packed_id_folded, to: &frame)
        if ((packed_id_folded >> 0) & 1) != 0 {
            appendVle(schema_len_derived, to: &frame)
        }
        if let held = source.schema {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
