// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_present_if_checked.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfChecked: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_seq`).
    public var hasSeq: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// Bit 1 of `flags` (flag `wants_trailer`).
    public var wantsTrailer: Bool {
        get { (flags & 0x2) != 0 }
        set { flags = newValue ? (flags | 0x2) : (flags & ~0x2) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var seq: UInt16? = nil

    /// On the wire only when `(flags & 0x1) != 0 || (flags & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 3, big-endian.
    public var trailer: UInt8? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, seq: UInt16?, trailer: UInt8?) {
        self.flags = flags
        self.seq = seq
        self.trailer = trailer
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfChecked {
        var reader = cursor
        var frame = CodecPresentIfChecked()
        frame.flags = try reader.readInteger(UInt8.self, .big)
        if (frame.flags & 0x1) != 0 {
            frame.seq = try reader.readInteger(UInt16.self, .big)
        }
        if (frame.flags & 0x1) != 0 || (frame.flags & 0x2) != 0 {
            frame.trailer = try reader.readInteger(UInt8.self, .big)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if (self.flags & 0x1) != 0 || (self.flags & 0x2) != 0 {
            if self.trailer == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.trailer != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendInteger(self.flags, to: &frame, .big)
        if let seq = self.seq {
            appendInteger(seq, to: &frame, .big)
        }
        if let trailer = self.trailer {
            appendInteger(trailer, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }
}
