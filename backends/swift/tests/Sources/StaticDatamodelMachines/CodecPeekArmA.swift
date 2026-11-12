// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_peek_arm_a.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPeekArmA: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 0 of `header` (flag `kind`).
    public var kind: Bool {
        get { (header & 0x1) != 0 }
        set { header = newValue ? (header | 0x1) : (header & ~0x1) }
    }

    /// The field at byte 1, big-endian.
    public var payload: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, payload: UInt8) {
        self.header = header
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPeekArmA {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecPeekArmA(
            header: try cursor.peekInteger(UInt8.self, at: 0, .big),
            payload: try cursor.peekInteger(UInt8.self, at: 1, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.header, into: &frame, at: 0, .big)
        storeInteger(self.payload, into: &frame, at: 1, .big)
        try sink.writeBytes(frame)
    }
}
