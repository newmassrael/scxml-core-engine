// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: codec_ext_encoding_info.scxml:44 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecExtEncodingInfo: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var combined_id: UInt32 = 0

    /// Bit 0 of `combined_id` (flag `has_schema`).
    public var hasSchema: Bool {
        get { (combined_id & 0x1) != 0 }
        set { combined_id = newValue ? (combined_id | 0x1) : (combined_id & ~0x1) }
    }

    /// The field at byte 1, big-endian.
    public var schema_size: UInt8 = 0

    /// On the wire only when `(combined_id & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `schema_size` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var schema: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(combined_id: UInt32, schema_size: UInt8, schema: [UInt8]?) {
        self.combined_id = combined_id
        self.schema_size = schema_size
        self.schema = schema
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecExtEncodingInfo {
        var reader = cursor
        var frame = CodecExtEncodingInfo()
        frame.combined_id = try reader.readVle(UInt32.self)
        frame.schema_size = try reader.readInteger(UInt8.self, .big)
        if (frame.combined_id & 0x1) != 0 {
            if frame.schema_size > 64 {
                throw CodecError.lengthOutOfRange
            }
            frame.schema = Array(try reader.readSlice(Int(clamping: frame.schema_size)))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.schema, value.count > 64 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendVle(self.combined_id, to: &frame)
        appendInteger(self.schema_size, to: &frame, .big)
        if let schema = self.schema {
            frame += schema
        }
        try sink.writeBytes(frame)
    }
}
