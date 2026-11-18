// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_simple_frame.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecSimpleFrame` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecSimpleFrame.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecSimpleFrame`, which conforms itself.
public protocol CodecSimpleFrameSource {
    var msgId: UInt8 { get }
    var length: UInt8 { get }
    var payload: UInt16 { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSimpleFrame: Equatable, Sendable, CodecSimpleFrameSource {
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

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecSimpleFrameSource` is written,
    /// with no copy into `CodecSimpleFrame`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecSimpleFrameSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.msgId, into: &frame, at: 0, .big)
        storeInteger(source.length, into: &frame, at: 1, .big)
        storeInteger(source.payload, into: &frame, at: 2, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
