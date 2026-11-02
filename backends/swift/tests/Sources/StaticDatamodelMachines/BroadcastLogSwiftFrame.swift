// SCE-GENERATED — DO NOT EDIT
// source-hash: ee6dc8a8311259cc746ef6014f59f29c1765ebca541134463c70d1f787755a9f
// SCE-MAP: broadcast_log_swift_frame.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct BroadcastLogSwiftFrame: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 11

    /// The field at byte 0, big-endian.
    public var sequence: UInt32 = 0

    /// The field at byte 4, big-endian.
    public var reading: UInt32 = 0

    /// The field at byte 8, big-endian.
    public var status: UInt16 = 0

    /// The field at byte 10, big-endian.
    public var level: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(sequence: UInt32, reading: UInt32, status: UInt16, level: UInt8) {
        self.sequence = sequence
        self.reading = reading
        self.status = status
        self.level = level
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> BroadcastLogSwiftFrame {
        _ = try cursor.peekSlice(frameLength)
        let frame = BroadcastLogSwiftFrame(
            sequence: try cursor.peekInteger(UInt32.self, at: 0, .big),
            reading: try cursor.peekInteger(UInt32.self, at: 4, .big),
            status: try cursor.peekInteger(UInt16.self, at: 8, .big),
            level: try cursor.peekInteger(UInt8.self, at: 10, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.sequence, into: &frame, at: 0, .big)
        storeInteger(self.reading, into: &frame, at: 4, .big)
        storeInteger(self.status, into: &frame, at: 8, .big)
        storeInteger(self.level, into: &frame, at: 10, .big)
        try sink.writeBytes(frame)
    }
}
