// SCE-GENERATED — DO NOT EDIT
// source-hash: 18ed40637a8a6093d84523309986c2bd9d249a2df22fcaea3b3a58a854842f31
// source: queue_conformance_event.scxml
// source: queue_conformance_scq.scxml
// source: queue_conformance_segmented.scxml
// source: queue_conformance_segmented_many.scxml
// source: queue_conformance_spsc.scxml
// SCE-MAP: queue_conformance_event.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `QueueConformanceEvent` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `QueueConformanceEvent.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `QueueConformanceEvent`, which conforms itself.
public protocol QueueConformanceEventSource {
    var sensor_id: UInt8 { get }
    var value: UInt16 { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct QueueConformanceEvent: Equatable, Sendable, QueueConformanceEventSource {
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

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `QueueConformanceEventSource` is written,
    /// with no copy into `QueueConformanceEvent`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: QueueConformanceEventSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.sensor_id, into: &frame, at: 0, .little)
        storeInteger(source.value, into: &frame, at: 1, .little)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
