// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: codec_present_if_tail.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfTail: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_payload`).
    public var hasPayload: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var payload: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, payload: [UInt8]?) {
        self.flags = flags
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfTail {
        var reader = cursor
        var frame = CodecPresentIfTail()
        frame.flags = try reader.readInteger(UInt8.self, .big)
        if (frame.flags & 0x1) != 0 {
            if reader.remaining > 64 {
                throw CodecError.lengthOutOfRange
            }
            frame.payload = Array(try reader.readSlice(reader.remaining))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.payload, value.count > 64 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.flags, to: &frame, .big)
        if let payload = self.payload {
            frame += payload
        }
        try sink.writeBytes(frame)
    }
}
