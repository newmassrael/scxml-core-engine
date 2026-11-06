// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: codec_declared_bounds_gated.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBoundsGated: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `G`).
    public var G: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var name_len: UInt8? = nil

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `name_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var name: String? = nil

    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var rest: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, name_len: UInt8?, name: String?, rest: [UInt8]) {
        self.flags = flags
        self.name_len = name_len
        self.name = name
        self.rest = rest
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecDeclaredBoundsGated {
        var reader = cursor
        var frame = CodecDeclaredBoundsGated()
        frame.flags = try reader.readInteger(UInt8.self, .big)
        if (frame.flags & 0x1) != 0 {
            frame.name_len = try reader.readInteger(UInt8.self, .big)
        }
        if (frame.flags & 0x1) != 0 {
            if (frame.name_len ?? 0) > 3 {
                throw CodecError.lengthOutOfRange
            }
            frame.name = try reader.readString(Int(clamping: (frame.name_len ?? 0)))
        }
        if reader.remaining > 2 {
            throw CodecError.lengthOutOfRange
        }
        frame.rest = Array(try reader.readSlice(reader.remaining))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.name, value.utf8.count > 3 {
            throw CodecError.lengthOutOfRange
        }
        if self.rest.count > 2 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.flags, to: &frame, .big)
        if let name_len = self.name_len {
            appendInteger(name_len, to: &frame, .big)
        }
        if let name = self.name {
            frame += Array(name.utf8)
        }
        frame += self.rest
        try sink.writeBytes(frame)
    }
}
