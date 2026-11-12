// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: codec_chain_flag_entry.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainFlagEntry: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 0 (flag `kind`). A value
    /// wider than the flag is cut to it when written.
    public var kind: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0xF) }
        set { header = (header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(wide & 0x1) == 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var short_value: UInt8? = nil

    /// On the wire only when `(wide & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 2, big-endian.
    public var long_value: UInt16? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, short_value: UInt8?, long_value: UInt16?) {
        self.header = header
        self.short_value = short_value
        self.long_value = long_value
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, wide: UInt8) throws(CodecError) -> CodecChainFlagEntry {
        var reader = cursor
        var frame = CodecChainFlagEntry()
        frame.header = try reader.readInteger(UInt8.self, .big)
        if (wide & 0x1) == 0 {
            frame.short_value = try reader.readInteger(UInt8.self, .big)
        }
        if (wide & 0x1) != 0 {
            frame.long_value = try reader.readInteger(UInt16.self, .big)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, wide: UInt8) throws(CodecError) {
        if (wide & 0x1) == 0 {
            if self.short_value == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.short_value != nil {
            throw CodecError.presentIfMismatch
        }
        if (wide & 0x1) != 0 {
            if self.long_value == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.long_value != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        if let short_value = self.short_value {
            appendInteger(short_value, to: &frame, .big)
        }
        if let long_value = self.long_value {
            appendInteger(long_value, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }
}
