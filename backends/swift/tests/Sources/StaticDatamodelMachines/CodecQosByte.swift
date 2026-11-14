// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_qos_byte.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecQosByte: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var qos: UInt8 = 0

    /// The 3 bits of `qos` from bit 0 (flag `priority`). A value
    /// wider than the flag is cut to it when written.
    public var priority: UInt8 {
        get { UInt8(truncatingIfNeeded: (qos >> 0) & 0x7) }
        set { qos = (qos & ~(0x7 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x7) << 0) }
    }

    /// Bit 3 of `qos` (flag `reliable`).
    public var reliable: Bool {
        get { (qos & 0x8) != 0 }
        set { qos = newValue ? (qos | 0x8) : (qos & ~0x8) }
    }

    /// The 2 bits of `qos` from bit 4 (flag `congestion`). A value
    /// wider than the flag is cut to it when written.
    public var congestion: UInt8 {
        get { UInt8(truncatingIfNeeded: (qos >> 4) & 0x3) }
        set { qos = (qos & ~(0x3 << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 4) }
    }

    /// Bit 6 of `qos` (flag `express`).
    public var express: Bool {
        get { (qos & 0x40) != 0 }
        set { qos = newValue ? (qos | 0x40) : (qos & ~0x40) }
    }

    /// Bit 7 of `qos` (flag `reserved`).
    public var reserved: Bool {
        get { (qos & 0x80) != 0 }
        set { qos = newValue ? (qos | 0x80) : (qos & ~0x80) }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(qos: UInt8) {
        self.qos = qos
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecQosByte {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecQosByte(
            qos: try cursor.peekInteger(UInt8.self, at: 0, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.qos, into: &frame, at: 0, .big)
        try sink.writeBytes(frame)
    }
}
