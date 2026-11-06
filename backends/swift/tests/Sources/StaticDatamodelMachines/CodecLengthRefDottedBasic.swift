// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_length_ref_dotted_basic.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLengthRefDottedBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// The 4 bits of `carrier` from bit 0 (flag `hdr`). A value
    /// wider than the flag is cut to it when written.
    public var hdr: UInt8 {
        get { UInt8(truncatingIfNeeded: (carrier >> 0) & 0xF) }
        set { carrier = (carrier & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// The 4 bits of `carrier` from bit 4 (flag `payload_len`). A value
    /// wider than the flag is cut to it when written.
    public var payloadLen: UInt8 {
        get { UInt8(truncatingIfNeeded: (carrier >> 4) & 0xF) }
        set { carrier = (carrier & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `carrier.payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var payload: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, payload: [UInt8]) {
        self.carrier = carrier
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLengthRefDottedBasic {
        var reader = cursor
        var frame = CodecLengthRefDottedBasic()
        frame.carrier = try reader.readInteger(UInt8.self, .big)
        frame.payload = Array(try reader.readSlice(Int(clamping: ((frame.carrier >> 4) & 0xF))))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.payload.count > 15 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.carrier, to: &frame, .big)
        frame += self.payload
        try sink.writeBytes(frame)
    }
}
