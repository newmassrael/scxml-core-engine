// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: codec_subbyte.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSubbyte: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The 3 bits of byte 0 from bit 5. A value wider than that is cut
    /// to it when written.
    public var priority: UInt8 = 0

    /// The 3 bits of byte 0 from bit 2. A value wider than that is cut
    /// to it when written.
    public var channel: UInt8 = 0

    /// The 2 bits of byte 0 from bit 0. A value wider than that is cut
    /// to it when written.
    public var direction: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(priority: UInt8, channel: UInt8, direction: UInt8) {
        self.priority = priority
        self.channel = channel
        self.direction = direction
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSubbyte {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecSubbyte(
            priority: try (cursor.peekInteger(UInt8.self, at: 0, .big) >> 5) & 0x7,
            channel: try (cursor.peekInteger(UInt8.self, at: 0, .big) >> 2) & 0x7,
            direction: try (cursor.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x3
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        frame[0] |= (self.priority & 0x7) << 5
        frame[0] |= (self.channel & 0x7) << 2
        frame[0] |= (self.direction & 0x3) << 0
        try sink.writeBytes(frame)
    }
}
