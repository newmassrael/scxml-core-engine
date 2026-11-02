// SCE-GENERATED — DO NOT EDIT
// source-hash: 7317770ec1283ca290af8bec72c8ce9016c9ca2036acfb8719dfd7159e505458
// SCE-MAP: codec_ext_attachment.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecExtAttachment: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var length: UInt8 = 0

    /// The bytes from byte 1 for as many as `length` says. The
    /// size the document bounds them to is not held here, as it is not by the other managed
    /// backends.
    public var body: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(length: UInt8, body: [UInt8]) {
        self.length = length
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecExtAttachment {
        _ = try cursor.peekSlice(frameLength)
        var frame = CodecExtAttachment()
        frame.length = try cursor.peekInteger(UInt8.self, at: 0, .big)
        // The count is the wire's: a huge one clamps and wraps to a negative end, which the cursor
        // answers as `needMoreBytes` like any frame that says more than it holds.
        let sceEnd = 1 &+ Int(clamping: frame.length)
        guard sceEnd >= 1 else { throw .needMoreBytes }
        frame.body = Array(try cursor.peekSlice(sceEnd).dropFirst(1))
        try cursor.advance(sceEnd)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.length, into: &frame, at: 0, .big)
        frame += self.body
        try sink.writeBytes(frame)
    }
}
