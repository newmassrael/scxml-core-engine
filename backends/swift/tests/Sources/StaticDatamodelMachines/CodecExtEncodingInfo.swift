// SCE-GENERATED — DO NOT EDIT
// source-hash: f0eb5a4f52c69f4f978f5a5e172a12cf2ff375056cd6ce9390d3db4b1509dd4b
// source: codec_ext_encoding_info.scxml
// SCE-MAP: codec_ext_encoding_info.scxml:44 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecExtEncodingInfo` the gates `combined_id.has_schema` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
public enum CodecExtEncodingInfoCombinedIdHasSchema: Equatable, Sendable {
    /// While combined_id.has_schema set.
    case hasSchema(schema: BoundedBytes<CodecExtEncodingInfo.SchemaBound>)
    /// While combined_id.has_schema clear.
    case notHasSchema(schema_size: UInt8)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .hasSchema: return 0
        case .notHasSchema: return 1
        }
    }
}

/// The elements a value of `CodecExtEncodingInfo` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecExtEncodingInfo.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecExtEncodingInfo`, which conforms itself.
public protocol CodecExtEncodingInfoSource {
    var combined_id: UInt32 { get }
    var combined_id_has_schema: CodecExtEncodingInfoCombinedIdHasSchema { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecExtEncodingInfo: Equatable, Sendable, CodecExtEncodingInfoSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `schema` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum SchemaBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 64
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var combined_id: UInt32 = 0

    /// Bit 0 of `combined_id` (flag `has_schema`).
    public var hasSchema: Bool {
        get { (combined_id & 0x1) != 0 }
        set { combined_id = newValue ? (combined_id | 0x1) : (combined_id & ~0x1) }
    }

    /// The elements the gates `combined_id.has_schema` decide together.
    public var combined_id_has_schema: CodecExtEncodingInfoCombinedIdHasSchema = .notHasSchema(schema_size: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(combined_id: UInt32, combined_id_has_schema: CodecExtEncodingInfoCombinedIdHasSchema) {
        self.combined_id = combined_id
        self.combined_id_has_schema = combined_id_has_schema
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecExtEncodingInfo {
        var reader = cursor
        let combined_id = try reader.readVle(UInt32.self)
        let schema_size = try reader.readInteger(UInt8.self, .big)
        var schema: BoundedBytes<CodecExtEncodingInfo.SchemaBound>? = nil
        if (combined_id & 0x1) != 0 {
            if schema_size > 64 {
                throw CodecError.lengthOutOfRange
            }
            schema = try BoundedBytes<CodecExtEncodingInfo.SchemaBound>(decoding: Array(try reader.readSlice(Int(clamping: schema_size))))
        }
        let combined_id_has_schema_group: CodecExtEncodingInfoCombinedIdHasSchema
        if ((combined_id >> 0) & 1) != 0 {
            combined_id_has_schema_group = .hasSchema(schema: schema!)
        } else {
            combined_id_has_schema_group = .notHasSchema(schema_size: schema_size)
        }
        cursor = reader
        return CodecExtEncodingInfo(combined_id: combined_id, combined_id_has_schema: combined_id_has_schema_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecExtEncodingInfoSource` is written,
    /// with no copy into `CodecExtEncodingInfo`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecExtEncodingInfoSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var schema_member: BoundedBytes<CodecExtEncodingInfo.SchemaBound>? = nil
        var schema_size_member: UInt8? = nil
        switch source.combined_id_has_schema {
        case .hasSchema(let schema): schema_member = schema
        case .notHasSchema(let schema_size): schema_size_member = schema_size
        }
        let schema_size_derived: UInt8 = schema_member != nil ? UInt8((schema_member?.byteCount ?? 0)) : (schema_size_member ?? 0)
        var combined_id_folded = source.combined_id
        combined_id_folded = (combined_id_folded & ~0x1) | ((schema_member != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendVle(combined_id_folded, to: &frame)
        appendInteger(schema_size_derived, to: &frame, .big)
        if let held = schema_member {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
