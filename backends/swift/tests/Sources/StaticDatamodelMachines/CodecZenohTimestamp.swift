// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_zenoh_timestamp.scxml:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohTimestamp: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var time: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var zid_len: UInt64 = 0

    /// The bytes, as many as `zid_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var zid: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(time: UInt64, zid_len: UInt64, zid: [UInt8]) {
        self.time = time
        self.zid_len = zid_len
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohTimestamp {
        var reader = cursor
        var frame = CodecZenohTimestamp()
        frame.time = try reader.readVle(UInt64.self)
        frame.zid_len = try reader.readVle(UInt64.self)
        if frame.zid_len > 16 {
            throw CodecError.lengthOutOfRange
        }
        frame.zid = Array(try reader.readSlice(Int(clamping: frame.zid_len)))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.zid.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendVle(self.time, to: &frame)
        appendVle(self.zid_len, to: &frame)
        frame += self.zid
        try sink.writeBytes(frame)
    }
}
