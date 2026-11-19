// SCE-GENERATED — DO NOT EDIT
// source-hash: c3deb2868eba47c48ed4cc9bdef13c2bd0d67d107cce5db651ad6678528685a5
// source: codec_chain_has_slice.scxml
// SCE-MAP: codec_chain_has_slice.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHasSlice` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasSlice.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasSlice`, which conforms itself.
public protocol CodecChainHasSliceSource {
    var kind: UInt16 { get }
    var bytes: BoundedBytes<CodecChainHasSlice.BytesBound> { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasSlice: Equatable, Sendable, CodecChainHasSliceSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `bytes` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum BytesBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 256
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var kind: UInt16 = 0

    /// The bytes, as many as `len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var bytes: BoundedBytes<CodecChainHasSlice.BytesBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(kind: UInt16, bytes: BoundedBytes<CodecChainHasSlice.BytesBound>) {
        self.kind = kind
        self.bytes = bytes
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasSlice {
        var reader = cursor
        let kind = try reader.readVle(UInt16.self)
        let len = try reader.readVle(UInt64.self)
        if len > 256 {
            throw CodecError.lengthOutOfRange
        }
        let bytes = try BoundedBytes<CodecChainHasSlice.BytesBound>(decoding: Array(try reader.readSlice(Int(clamping: len))))
        cursor = reader
        return CodecChainHasSlice(kind: kind, bytes: bytes)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasSliceSource` is written,
    /// with no copy into `CodecChainHasSlice`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasSliceSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let len_derived: UInt64 = UInt64(source.bytes.byteCount)
        var frame: [UInt8] = []
        appendVle(source.kind, to: &frame)
        appendVle(len_derived, to: &frame)
        source.bytes.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
