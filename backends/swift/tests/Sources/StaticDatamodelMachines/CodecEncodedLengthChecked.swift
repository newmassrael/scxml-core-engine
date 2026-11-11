// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_encoded_length_checked.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEncodedLengthChecked: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecLengthShared.frameLength

    /// The field at byte 0, big-endian.
    public var inner_size: UInt16 = 0

    /// A `CodecLengthShared` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var inner: CodecLengthShared = CodecLengthShared()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(inner_size: UInt16, inner: CodecLengthShared) {
        self.inner_size = inner_size
        self.inner = inner
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecEncodedLengthChecked {
        var reader = cursor
        var frame = CodecEncodedLengthChecked()
        frame.inner_size = try reader.readInteger(UInt16.self, .big)
        let inner_inner_length = Int(clamping: frame.inner_size)
        var inner_inner = SceCursor(Array(try reader.peekSlice(inner_inner_length)))
        frame.inner = try CodecLengthShared.decode(&inner_inner)
        try reader.advance(inner_inner_length)
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.inner_size, to: &frame, .big)
        do {
            var piece = BytesSink()
            try self.inner.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
