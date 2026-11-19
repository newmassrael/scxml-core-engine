// SCE-GENERATED — DO NOT EDIT
// source-hash: c4f4e96f9f783fdc0083b868089b686d8c2c336b908d00fc7fb2646531a067f3
// source: codec_chain_has_marker_slice.scxml
// SCE-MAP: codec_chain_has_marker_slice.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHasMarkerSlice` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasMarkerSlice.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasMarkerSlice`, which conforms itself.
public protocol CodecChainHasMarkerSliceSource {
    var kind: UInt16 { get }
    var data: BoundedBytes<CodecChainHasMarkerSlice.DataBound> { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasMarkerSlice: Equatable, Sendable, CodecChainHasMarkerSliceSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `data` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum DataBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 256
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var kind: UInt16 = 0

    /// The bytes, as many as `size` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var data: BoundedBytes<CodecChainHasMarkerSlice.DataBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(kind: UInt16, data: BoundedBytes<CodecChainHasMarkerSlice.DataBound>) {
        self.kind = kind
        self.data = data
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasMarkerSlice {
        var reader = cursor
        let kind = try reader.readVle(UInt16.self)
        let size = try reader.readVle(UInt64.self)
        if size > 256 {
            throw CodecError.lengthOutOfRange
        }
        let data = try BoundedBytes<CodecChainHasMarkerSlice.DataBound>(decoding: Array(try reader.readSlice(Int(clamping: size))))
        cursor = reader
        return CodecChainHasMarkerSlice(kind: kind, data: data)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasMarkerSliceSource` is written,
    /// with no copy into `CodecChainHasMarkerSlice`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasMarkerSliceSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let size_derived: UInt64 = UInt64(source.data.byteCount)
        var frame: [UInt8] = []
        appendVle(source.kind, to: &frame)
        appendVle(size_derived, to: &frame)
        source.data.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
