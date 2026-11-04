// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d95af6fbc211ea1c5b1e249d16b701ebad41657a9f4b213349f1c668ce4ccbb
// SCE-MAP: broadcast_log_conformance_flagged_event.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct BroadcastLogConformanceFlaggedEvent: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, little-endian.
    public var sensor_id: UInt8 = 0

    /// On the wire only when `(has_value & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, little-endian.
    public var value: UInt16? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(sensor_id: UInt8, value: UInt16?) {
        self.sensor_id = sensor_id
        self.value = value
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, has_value: UInt8) throws(CodecError) -> BroadcastLogConformanceFlaggedEvent {
        var reader = cursor
        var frame = BroadcastLogConformanceFlaggedEvent()
        frame.sensor_id = try reader.readInteger(UInt8.self, .little)
        if (has_value & 0x1) != 0 {
            frame.value = try reader.readInteger(UInt16.self, .little)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, has_value: UInt8) throws(CodecError) {
        if (has_value & 0x1) != 0 {
            if self.value == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.value != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendInteger(self.sensor_id, to: &frame, .little)
        if let value = self.value {
            appendInteger(value, to: &frame, .little)
        }
        try sink.writeBytes(frame)
    }
}
