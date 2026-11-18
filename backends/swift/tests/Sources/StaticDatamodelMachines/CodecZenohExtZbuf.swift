// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_ext_zbuf.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohExtZbuf` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohExtZbuf.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohExtZbuf`, which conforms itself.
public protocol CodecZenohExtZbufSource {
    var value: BoundedBytes<CodecZenohExtZbuf.ValueBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohExtZbuf: Equatable, Sendable, CodecZenohExtZbufSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `value` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ValueBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 4294967295
    }

    /// The bytes, as many as `value_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var value: BoundedBytes<CodecZenohExtZbuf.ValueBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(value: BoundedBytes<CodecZenohExtZbuf.ValueBound>) {
        self.value = value
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohExtZbuf {
        var reader = cursor
        let value_len = try reader.readVle(UInt64.self)
        if value_len > 4294967295 {
            throw CodecError.lengthOutOfRange
        }
        let value = try BoundedBytes<CodecZenohExtZbuf.ValueBound>(decoding: Array(try reader.readSlice(Int(clamping: value_len))))
        cursor = reader
        return CodecZenohExtZbuf(value: value)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohExtZbufSource` is written,
    /// with no copy into `CodecZenohExtZbuf`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohExtZbufSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let value_len_derived: UInt64 = UInt64(source.value.byteCount)
        var frame: [UInt8] = []
        appendVle(value_len_derived, to: &frame)
        source.value.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
