// SCE-GENERATED — DO NOT EDIT
// source-hash: 7317770ec1283ca290af8bec72c8ce9016c9ca2036acfb8719dfd7159e505458
// SCE-MAP: codec_scout_zid_body.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecScoutZidBody: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var zid_len_m1: UInt8 = 0

    /// The bytes from byte 1 for as many as `zid_len_m1` says, plus 1. The
    /// size the document bounds them to is not held here, as it is not by the other managed
    /// backends.
    public var zid: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(zid_len_m1: UInt8, zid: [UInt8]) {
        self.zid_len_m1 = zid_len_m1
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecScoutZidBody {
        _ = try cursor.peekSlice(frameLength)
        var frame = CodecScoutZidBody()
        frame.zid_len_m1 = try cursor.peekInteger(UInt8.self, at: 0, .big)
        // The count is the wire's: a huge one clamps and wraps to a negative end, which the cursor
        // answers as `needMoreBytes` like any frame that says more than it holds.
        let sceEnd = 1 &+ Int(clamping: frame.zid_len_m1) &+ (1)
        guard sceEnd >= 1 else { throw .needMoreBytes }
        frame.zid = Array(try cursor.peekSlice(sceEnd).dropFirst(1))
        try cursor.advance(sceEnd)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.zid_len_m1, into: &frame, at: 0, .big)
        frame += self.zid
        try sink.writeBytes(frame)
    }
}
