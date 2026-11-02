// SCE-GENERATED — DO NOT EDIT
// source-hash: 7317770ec1283ca290af8bec72c8ce9016c9ca2036acfb8719dfd7159e505458
// SCE-MAP: codec_tlv_entry.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecTlvEntry: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var entry_type: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var entry_len: UInt8 = 0

    /// The bytes from byte 2 for as many as `entry_len` says. The
    /// size the document bounds them to is not held here, as it is not by the other managed
    /// backends.
    public var entry_body: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(entry_type: UInt8, entry_len: UInt8, entry_body: [UInt8]) {
        self.entry_type = entry_type
        self.entry_len = entry_len
        self.entry_body = entry_body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecTlvEntry {
        _ = try cursor.peekSlice(frameLength)
        var frame = CodecTlvEntry()
        frame.entry_type = try cursor.peekInteger(UInt8.self, at: 0, .big)
        frame.entry_len = try cursor.peekInteger(UInt8.self, at: 1, .big)
        // The count is the wire's: a huge one clamps and wraps to a negative end, which the cursor
        // answers as `needMoreBytes` like any frame that says more than it holds.
        let sceEnd = 2 &+ Int(clamping: frame.entry_len)
        guard sceEnd >= 2 else { throw .needMoreBytes }
        frame.entry_body = Array(try cursor.peekSlice(sceEnd).dropFirst(2))
        try cursor.advance(sceEnd)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.entry_type, into: &frame, at: 0, .big)
        storeInteger(self.entry_len, into: &frame, at: 1, .big)
        frame += self.entry_body
        try sink.writeBytes(frame)
    }
}
