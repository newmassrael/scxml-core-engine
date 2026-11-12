// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: queue_conformance_event.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct QueueConformanceEvent: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, little-endian.
    public var sensor_id: UInt8 = 0

    /// The field at byte 1, little-endian.
    public var value: UInt16 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(sensor_id: UInt8, value: UInt16) {
        self.sensor_id = sensor_id
        self.value = value
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> QueueConformanceEvent {
        _ = try cursor.peekSlice(frameLength)
        let frame = QueueConformanceEvent(
            sensor_id: try cursor.peekInteger(UInt8.self, at: 0, .little),
            value: try cursor.peekInteger(UInt16.self, at: 1, .little)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.sensor_id, into: &frame, at: 0, .little)
        storeInteger(self.value, into: &frame, at: 1, .little)
        try sink.writeBytes(frame)
    }
}
