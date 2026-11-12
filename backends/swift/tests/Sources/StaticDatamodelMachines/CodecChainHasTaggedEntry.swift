// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_chain_has_tagged_entry.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasTaggedEntry: Equatable, Sendable {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, big-endian.
    public var entry_type: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var ctl: UInt8 = 0

    /// Bit 0 of `ctl` (flag `more`).
    public var more: Bool {
        get { (ctl & 0x1) != 0 }
        set { ctl = newValue ? (ctl | 0x1) : (ctl & ~0x1) }
    }

    /// The field at byte 2, big-endian.
    public var body: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(entry_type: UInt8, ctl: UInt8, body: UInt8) {
        self.entry_type = entry_type
        self.ctl = ctl
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasTaggedEntry {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecChainHasTaggedEntry(
            entry_type: try cursor.peekInteger(UInt8.self, at: 0, .big),
            ctl: try cursor.peekInteger(UInt8.self, at: 1, .big),
            body: try cursor.peekInteger(UInt8.self, at: 2, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(self.entry_type, into: &frame, at: 0, .big)
        storeInteger(self.ctl, into: &frame, at: 1, .big)
        storeInteger(self.body, into: &frame, at: 2, .big)
        try sink.writeBytes(frame)
    }
}
