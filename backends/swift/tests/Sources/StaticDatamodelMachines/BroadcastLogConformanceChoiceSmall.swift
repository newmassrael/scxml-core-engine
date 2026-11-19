// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d95af6fbc211ea1c5b1e249d16b701ebad41657a9f4b213349f1c668ce4ccbb
// source: broadcast_log_conformance_choice.scxml
// source: broadcast_log_conformance_choice_large.scxml
// source: broadcast_log_conformance_choice_small.scxml
// source: broadcast_log_conformance_chosen.scxml
// source: broadcast_log_conformance_flagged.scxml
// source: broadcast_log_conformance_flagged_event.scxml
// SCE-MAP: broadcast_log_conformance_choice_small.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `BroadcastLogConformanceChoiceSmall` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `BroadcastLogConformanceChoiceSmall.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `BroadcastLogConformanceChoiceSmall`, which conforms itself.
public protocol BroadcastLogConformanceChoiceSmallSource {
    var payload: UInt8 { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct BroadcastLogConformanceChoiceSmall: Equatable, Sendable, BroadcastLogConformanceChoiceSmallSource {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, little-endian.
    public var payload: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(payload: UInt8) {
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> BroadcastLogConformanceChoiceSmall {
        _ = try cursor.peekSlice(frameLength)
        let frame = BroadcastLogConformanceChoiceSmall(
            payload: try cursor.peekInteger(UInt8.self, at: 0, .little)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `BroadcastLogConformanceChoiceSmallSource` is written,
    /// with no copy into `BroadcastLogConformanceChoiceSmall`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: BroadcastLogConformanceChoiceSmallSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.payload, into: &frame, at: 0, .little)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
