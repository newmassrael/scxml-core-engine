// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner.scxml:65 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclExtKeyexprInner: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var inner_header: UInt8 = 0

    /// Bit 0 of `inner_header` (flag `N`).
    public var N: Bool {
        get { (inner_header & 0x1) != 0 }
        set { inner_header = newValue ? (inner_header | 0x1) : (inner_header & ~0x1) }
    }

    /// Bit 1 of `inner_header` (flag `M`).
    public var M: Bool {
        get { (inner_header & 0x2) != 0 }
        set { inner_header = newValue ? (inner_header | 0x2) : (inner_header & ~0x2) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt64 = 0

    /// On the wire only when `(inner_header & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var suffix: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(inner_header: UInt8, id: UInt64, suffix: [UInt8]?) {
        self.inner_header = inner_header
        self.id = id
        self.suffix = suffix
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclExtKeyexprInner {
        var reader = cursor
        var frame = CodecZenohDeclExtKeyexprInner()
        frame.inner_header = try reader.readInteger(UInt8.self, .big)
        frame.id = try reader.readVle(UInt64.self)
        if (frame.inner_header & 0x1) != 0 {
            if reader.remaining > 65535 {
                throw CodecError.lengthOutOfRange
            }
            frame.suffix = Array(try reader.readSlice(reader.remaining))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.suffix, value.count > 65535 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.inner_header, to: &frame, .big)
        appendVle(self.id, to: &frame)
        if let suffix = self.suffix {
            frame += suffix
        }
        try sink.writeBytes(frame)
    }
}
