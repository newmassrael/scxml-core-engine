// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_zenoh_source_info.scxml:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohSourceInfo: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 4 (flag `zidlen_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidlenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 4) & 0xF) }
        set { header = (header & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `header.zidlen_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var zid: [UInt8] = []

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var eid: UInt32 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var sn: UInt32 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, zid: [UInt8], eid: UInt32, sn: UInt32) {
        self.header = header
        self.zid = zid
        self.eid = eid
        self.sn = sn
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohSourceInfo {
        var reader = cursor
        var frame = CodecZenohSourceInfo()
        frame.header = try reader.readInteger(UInt8.self, .big)
        frame.zid = Array(try reader.readSlice(Int(clamping: ((frame.header >> 4) & 0xF)) &+ (1)))
        frame.eid = try reader.readVle(UInt32.self)
        frame.sn = try reader.readVle(UInt32.self)
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.zid.count < 1 || self.zid.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        frame += self.zid
        appendVle(self.eid, to: &frame)
        appendVle(self.sn, to: &frame)
        try sink.writeBytes(frame)
    }
}
