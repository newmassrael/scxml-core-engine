// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_every_arm_says_near.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEveryArmSaysNear: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var id: UInt8 = 0

    /// On the wire only when `(N & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var suffix_len: UInt8? = nil

    /// On the wire only when `(N & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `suffix_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var suffix: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt8, suffix_len: UInt8?, suffix: [UInt8]?) {
        self.id = id
        self.suffix_len = suffix_len
        self.suffix = suffix
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, N: UInt8) throws(CodecError) -> CodecEveryArmSaysNear {
        var reader = cursor
        var frame = CodecEveryArmSaysNear()
        frame.id = try reader.readInteger(UInt8.self, .big)
        if (N & 0x1) != 0 {
            frame.suffix_len = try reader.readInteger(UInt8.self, .big)
        }
        if (N & 0x1) != 0 {
            if (frame.suffix_len ?? 0) > 8 {
                throw CodecError.lengthOutOfRange
            }
            frame.suffix = Array(try reader.readSlice(Int(clamping: (frame.suffix_len ?? 0))))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, N: UInt8) throws(CodecError) {
        if (N & 0x1) != 0 {
            if self.suffix == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.suffix != nil {
            throw CodecError.presentIfMismatch
        }
        if let value = self.suffix, value.count > 8 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.id, to: &frame, .big)
        if let suffix_len = self.suffix_len {
            appendInteger(suffix_len, to: &frame, .big)
        }
        if let suffix = self.suffix {
            frame += suffix
        }
        try sink.writeBytes(frame)
    }
}
