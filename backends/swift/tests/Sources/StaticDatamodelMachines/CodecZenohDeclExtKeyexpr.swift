// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_zenoh_decl_ext_keyexpr.scxml:89 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclExtKeyexpr: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecZenohDeclExtKeyexprInner.frameLength

    /// The field at byte 0, big-endian.
    public var outer_header: UInt8 = 0

    /// The 4 bits of `outer_header` from bit 0 (flag `ext_id`). A value
    /// wider than the flag is cut to it when written.
    public var extId: UInt8 {
        get { UInt8(truncatingIfNeeded: (outer_header >> 0) & 0xF) }
        set { outer_header = (outer_header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 4 of `outer_header` (flag `M`).
    public var M: Bool {
        get { (outer_header & 0x10) != 0 }
        set { outer_header = newValue ? (outer_header | 0x10) : (outer_header & ~0x10) }
    }

    /// The 2 bits of `outer_header` from bit 5 (flag `enc`). A value
    /// wider than the flag is cut to it when written.
    public var enc: UInt8 {
        get { UInt8(truncatingIfNeeded: (outer_header >> 5) & 0x3) }
        set { outer_header = (outer_header & ~(0x3 << 5)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 5) }
    }

    /// Bit 7 of `outer_header` (flag `Z`).
    public var Z: Bool {
        get { (outer_header & 0x80) != 0 }
        set { outer_header = newValue ? (outer_header | 0x80) : (outer_header & ~0x80) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var total_length: UInt64 = 0

    /// A `CodecZenohDeclExtKeyexprInner` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var inner: CodecZenohDeclExtKeyexprInner = CodecZenohDeclExtKeyexprInner()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(outer_header: UInt8, total_length: UInt64, inner: CodecZenohDeclExtKeyexprInner) {
        self.outer_header = outer_header
        self.total_length = total_length
        self.inner = inner
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclExtKeyexpr {
        var reader = cursor
        var frame = CodecZenohDeclExtKeyexpr()
        frame.outer_header = try reader.readInteger(UInt8.self, .big)
        frame.total_length = try reader.readVle(UInt64.self)
        let inner_inner_length = Int(clamping: frame.total_length)
        var inner_inner = SceCursor(Array(try reader.peekSlice(inner_inner_length)))
        frame.inner = try CodecZenohDeclExtKeyexprInner.decode(&inner_inner)
        try reader.advance(inner_inner_length)
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.outer_header, to: &frame, .big)
        appendVle(self.total_length, to: &frame)
        do {
            var piece = BytesSink()
            try self.inner.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
