// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_length_stated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLengthStated: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_alpha`).
    public var hasAlpha: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// Bit 1 of `flags` (flag `has_beta`).
    public var hasBeta: Bool {
        get { (flags & 0x2) != 0 }
        set { flags = newValue ? (flags | 0x2) : (flags & ~0x2) }
    }

    /// The field at byte 1, big-endian.
    public var span_len: UInt8 = 0

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `span_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var alpha: [UInt8]? = nil

    /// On the wire only when `(flags & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `span_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var beta: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, span_len: UInt8, alpha: [UInt8]?, beta: [UInt8]?) {
        self.flags = flags
        self.span_len = span_len
        self.alpha = alpha
        self.beta = beta
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLengthStated {
        var reader = cursor
        var frame = CodecLengthStated()
        frame.flags = try reader.readInteger(UInt8.self, .big)
        frame.span_len = try reader.readInteger(UInt8.self, .big)
        if (frame.flags & 0x1) != 0 {
            frame.alpha = Array(try reader.readSlice(Int(clamping: frame.span_len)))
        }
        if (frame.flags & 0x2) != 0 {
            frame.beta = Array(try reader.readSlice(Int(clamping: frame.span_len)))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.alpha, value.count > 255 {
            throw CodecError.lengthOutOfRange
        }
        if let value = self.beta, value.count > 255 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.flags, to: &frame, .big)
        appendInteger(self.span_len, to: &frame, .big)
        if let alpha = self.alpha {
            frame += alpha
        }
        if let beta = self.beta {
            frame += beta
        }
        try sink.writeBytes(frame)
    }
}
