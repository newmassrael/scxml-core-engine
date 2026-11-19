// SCE-GENERATED — DO NOT EDIT
// source-hash: 3b83e0f30691d46330790934f7083c0d39d4be2c314d38f6b9edab5d4250c383
// source: codec_origin_leaf.scxml
// SCE-MAP: codec_origin_leaf.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecOriginLeaf` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecOriginLeaf.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecOriginLeaf`, which conforms itself.
public protocol CodecOriginLeafSource {
    var data: BoundedBytes<CodecOriginLeaf.DataBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOriginLeaf: Equatable, Sendable, CodecOriginLeafSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `data` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum DataBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 16
    }

    /// The bytes, as many as `len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var data: BoundedBytes<CodecOriginLeaf.DataBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(data: BoundedBytes<CodecOriginLeaf.DataBound>) {
        self.data = data
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOriginLeaf {
        var reader = cursor
        let len = try reader.readInteger(UInt8.self, .big)
        if len > 16 {
            throw CodecError.lengthOutOfRange
        }
        let data = try BoundedBytes<CodecOriginLeaf.DataBound>(decoding: Array(try reader.readSlice(Int(clamping: len))))
        cursor = reader
        return CodecOriginLeaf(data: data)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecOriginLeafSource` is written,
    /// with no copy into `CodecOriginLeaf`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecOriginLeafSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let len_derived: UInt8 = UInt8(source.data.byteCount)
        var frame = [UInt8](repeating: 0, count: 1)
        storeInteger(len_derived, into: &frame, at: 0, .big)
        source.data.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
