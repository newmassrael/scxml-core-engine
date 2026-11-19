// SCE-GENERATED — DO NOT EDIT
// source-hash: 77cfddcffd58942b5a0e39f78a03cdca538c456a176d61819754121240a81474
// source: codec_embed_basic.scxml
// SCE-MAP: codec_embed_basic.scxml:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecEmbedBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecEmbedBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecEmbedBasic`, which conforms itself.
public protocol CodecEmbedBasicSource {
    /// What `locator` is presented as: a source of `CodecZenohLocator`, written by its encode.
    associatedtype Locator_Source: CodecZenohLocatorSource
    var tag: UInt8 { get }
    var locator: Locator_Source { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEmbedBasic: Equatable, Sendable, CodecEmbedBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohLocator.frameLength

    /// The value presents `locator` as a value of its own codec.
    public typealias Locator_Source = CodecZenohLocator

    /// The field at byte 0, big-endian.
    public var tag: UInt8 = 0

    /// A `CodecZenohLocator` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var locator: CodecZenohLocator = CodecZenohLocator()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(tag: UInt8, locator: Locator_Source) {
        self.tag = tag
        self.locator = locator
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecEmbedBasic {
        var reader = cursor
        let tag = try reader.readInteger(UInt8.self, .big)
        let locator = try CodecZenohLocator.decode(&reader)
        cursor = reader
        return CodecEmbedBasic(tag: tag, locator: locator)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecEmbedBasicSource` is written,
    /// with no copy into `CodecEmbedBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecEmbedBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var locator_piece = BytesSink()
        try CodecZenohLocator.encodeSource(source.locator, into: &locator_piece)
        let locator_bytes = locator_piece.bytes
        var frame: [UInt8] = []
        appendInteger(source.tag, to: &frame, .big)
        frame += locator_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
