// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_timestamp_ext.scxml:48 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohTimestampExt` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohTimestampExt.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohTimestampExt`, which conforms itself.
public protocol CodecZenohTimestampExtSource {
    /// What `ts` is presented as: a source of `CodecZenohTimestamp`, written by its encode.
    associatedtype Ts_Source: CodecZenohTimestampSource
    var ts: Ts_Source { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohTimestampExt: Equatable, Sendable, CodecZenohTimestampExtSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohTimestamp.frameLength

    /// The value presents `ts` as a value of its own codec.
    public typealias Ts_Source = CodecZenohTimestamp

    /// A `CodecZenohTimestamp` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var ts: CodecZenohTimestamp = CodecZenohTimestamp()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(ts: Ts_Source) {
        self.ts = ts
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohTimestampExt {
        var reader = cursor
        let ext_size = try reader.readVle(UInt64.self)
        let ts_inner_length = Int(clamping: ext_size)
        var ts_inner = SceCursor(Array(try reader.peekSlice(ts_inner_length)))
        let ts = try CodecZenohTimestamp.decode(&ts_inner)
        try reader.advance(ts_inner_length)
        cursor = reader
        return CodecZenohTimestampExt(ts: ts)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohTimestampExtSource` is written,
    /// with no copy into `CodecZenohTimestampExt`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohTimestampExtSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var ts_piece = BytesSink()
        try CodecZenohTimestamp.encodeSource(source.ts, into: &ts_piece)
        let ts_bytes = ts_piece.bytes
        let ext_size_derived: UInt64 = UInt64(ts_bytes.count)
        var frame: [UInt8] = []
        appendVle(ext_size_derived, to: &frame)
        frame += ts_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
