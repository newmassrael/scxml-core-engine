// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_encoded_length_checked.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecEncodedLengthChecked` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecEncodedLengthChecked.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecEncodedLengthChecked`, which conforms itself.
public protocol CodecEncodedLengthCheckedSource {
    /// What `inner` is presented as: a source of `CodecLengthShared`, written by its encode.
    associatedtype Inner_Source: CodecLengthSharedSource
    var inner: Inner_Source { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEncodedLengthChecked: Equatable, Sendable, CodecEncodedLengthCheckedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecLengthShared.frameLength

    /// The value presents `inner` as a value of its own codec.
    public typealias Inner_Source = CodecLengthShared

    /// A `CodecLengthShared` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var inner: CodecLengthShared = CodecLengthShared()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(inner: Inner_Source) {
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
        let inner_size = try reader.readInteger(UInt16.self, .big)
        let inner_inner_length = Int(clamping: inner_size)
        var inner_inner = SceCursor(Array(try reader.peekSlice(inner_inner_length)))
        let inner = try CodecLengthShared.decode(&inner_inner)
        try reader.advance(inner_inner_length)
        cursor = reader
        return CodecEncodedLengthChecked(inner: inner)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecEncodedLengthCheckedSource` is written,
    /// with no copy into `CodecEncodedLengthChecked`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecEncodedLengthCheckedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var inner_piece = BytesSink()
        try CodecLengthShared.encodeSource(source.inner, into: &inner_piece)
        let inner_bytes = inner_piece.bytes
        if inner_bytes.count > 0xFFFF { throw CodecError.lengthOutOfRange }
        let inner_size_derived: UInt16 = UInt16(inner_bytes.count)
        var frame: [UInt8] = []
        appendInteger(inner_size_derived, to: &frame, .big)
        frame += inner_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
