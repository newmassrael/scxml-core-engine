// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: codec_scout_zid_body.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecScoutZidBody: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var zid_len_m1: UInt8 = 0

    /// The bytes, as many as `zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var zid: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(zid_len_m1: UInt8, zid: [UInt8]) {
        self.zid_len_m1 = zid_len_m1
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecScoutZidBody {
        var reader = cursor
        var frame = CodecScoutZidBody()
        frame.zid_len_m1 = try reader.readInteger(UInt8.self, .big)
        if frame.zid_len_m1 > 15 {
            throw CodecError.lengthOutOfRange
        }
        frame.zid = Array(try reader.readSlice(Int(clamping: frame.zid_len_m1) &+ (1)))
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
        appendInteger(self.zid_len_m1, to: &frame, .big)
        frame += self.zid
        try sink.writeBytes(frame)
    }
}
