// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: subscription_entry.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct SubscriptionEntry: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 8

    /// The field at byte 4, big-endian.
    public var callback_id: UInt32 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(callback_id: UInt32) {
        self.callback_id = callback_id
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> SubscriptionEntry {
        _ = try cursor.peekSlice(frameLength)
        let frame = SubscriptionEntry(
            callback_id: try cursor.peekInteger(UInt32.self, at: 4, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.callback_id, into: &frame, at: 4, .big)
        try sink.writeBytes(frame)
    }
}
