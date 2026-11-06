// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: codec_simple_frame.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSimpleFrame: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 4

    /// The field at byte 0, big-endian.
    public var msgId: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var length: UInt8 = 0

    /// The field at byte 2, big-endian.
    public var payload: UInt16 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(msgId: UInt8, length: UInt8, payload: UInt16) {
        self.msgId = msgId
        self.length = length
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSimpleFrame {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecSimpleFrame(
            msgId: try cursor.peekInteger(UInt8.self, at: 0, .big),
            length: try cursor.peekInteger(UInt8.self, at: 1, .big),
            payload: try cursor.peekInteger(UInt16.self, at: 2, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.msgId, into: &frame, at: 0, .big)
        storeInteger(self.length, into: &frame, at: 1, .big)
        storeInteger(self.payload, into: &frame, at: 2, .big)
        try sink.writeBytes(frame)
    }
}
