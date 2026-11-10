// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_zenoh_scout.scxml:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohScout: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var cbyte: UInt8 = 0

    /// The 3 bits of `cbyte` from bit 0 (flag `what`). A value
    /// wider than the flag is cut to it when written.
    public var what: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 0) & 0x7) }
        set { cbyte = (cbyte & ~(0x7 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x7) << 0) }
    }

    /// Bit 3 of `cbyte` (flag `I`).
    public var I: Bool {
        get { (cbyte & 0x8) != 0 }
        set { cbyte = newValue ? (cbyte | 0x8) : (cbyte & ~0x8) }
    }

    /// The 4 bits of `cbyte` from bit 4 (flag `zid_len_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidLenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 4) & 0xF) }
        set { cbyte = (cbyte & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// On the wire only when `(cbyte & 0x8) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cbyte.zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var zid: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: [UInt8]?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohScout {
        var reader = cursor
        var frame = CodecZenohScout()
        frame.version = try reader.readInteger(UInt8.self, .big)
        frame.cbyte = try reader.readInteger(UInt8.self, .big)
        if (frame.cbyte & 0x8) != 0 {
            frame.zid = Array(try reader.readSlice(Int(clamping: ((frame.cbyte >> 4) & 0xF)) &+ (1)))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.zid, value.count < 1 || value.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.version, to: &frame, .big)
        appendInteger(self.cbyte, to: &frame, .big)
        if let zid = self.zid {
            frame += zid
        }
        try sink.writeBytes(frame)
    }
}
