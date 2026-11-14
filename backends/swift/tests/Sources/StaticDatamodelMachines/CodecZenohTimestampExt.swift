// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: codec_zenoh_timestamp_ext.scxml:48 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohTimestampExt: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohTimestamp.frameLength

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var ext_size: UInt64 = 0

    /// A `CodecZenohTimestamp` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var ts: CodecZenohTimestamp = CodecZenohTimestamp()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(ext_size: UInt64, ts: CodecZenohTimestamp) {
        self.ext_size = ext_size
        self.ts = ts
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohTimestampExt {
        var reader = cursor
        var frame = CodecZenohTimestampExt()
        frame.ext_size = try reader.readVle(UInt64.self)
        let ts_inner_length = Int(clamping: frame.ext_size)
        var ts_inner = SceCursor(Array(try reader.peekSlice(ts_inner_length)))
        frame.ts = try CodecZenohTimestamp.decode(&ts_inner)
        try reader.advance(ts_inner_length)
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendVle(self.ext_size, to: &frame)
        do {
            var piece = BytesSink()
            try self.ts.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
