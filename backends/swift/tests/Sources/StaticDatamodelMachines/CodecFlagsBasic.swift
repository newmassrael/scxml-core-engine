// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: codec_flags_basic.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecFlagsBasic: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 7 of `header` (flag `reliable`).
    public var reliable: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// Bit 6 of `header` (flag `more`).
    public var more: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 5 of `header` (flag `drop`).
    public var drop: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 4 of `header` (flag `first`).
    public var first: Bool {
        get { (header & 0x10) != 0 }
        set { header = newValue ? (header | 0x10) : (header & ~0x10) }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8) {
        self.header = header
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecFlagsBasic {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecFlagsBasic(
            header: try cursor.peekInteger(UInt8.self, at: 0, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.header, into: &frame, at: 0, .big)
        try sink.writeBytes(frame)
    }
}
