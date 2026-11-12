// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_optional_field_attributes.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOptionalFieldAttributes: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, little-endian.
    public var status: UInt8 = 0

    /// Bit 2 of `status` (flag `ready`).
    public var ready: Bool {
        get { (status & 0x4) != 0 }
        set { status = newValue ? (status | 0x4) : (status & ~0x4) }
    }

    /// Bit 3 of `status` (flag `fault`).
    public var fault: Bool {
        get { (status & 0x8) != 0 }
        set { status = newValue ? (status | 0x8) : (status & ~0x8) }
    }

    /// The 12 bits from bit 4 of the 2 bytes from byte 1, read as one
    /// big-endian word. A value wider than that is cut to it when written.
    public var reading: UInt16 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(status: UInt8, reading: UInt16) {
        self.status = status
        self.reading = reading
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOptionalFieldAttributes {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecOptionalFieldAttributes(
            status: try cursor.peekInteger(UInt8.self, at: 0, .little),
            reading: try (cursor.peekInteger(UInt16.self, at: 1, .big) >> 4) & 0xFFF
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.status, into: &frame, at: 0, .little)
        frame[1] |= UInt8(truncatingIfNeeded: (self.reading >> 4) & 0xFF) << 0
        frame[2] |= UInt8(truncatingIfNeeded: self.reading & 0xF) << 4
        try sink.writeBytes(frame)
    }
}
