// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_locator.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohLocator` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohLocator.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohLocator`, which conforms itself.
public protocol CodecZenohLocatorSource {
    var locator: BoundedText<CodecZenohLocator.LocatorBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohLocator: Equatable, Sendable, CodecZenohLocatorSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `locator` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum LocatorBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 255
    }

    /// The text of as many UTF-8 bytes as `locator_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var locator: BoundedText<CodecZenohLocator.LocatorBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(locator: BoundedText<CodecZenohLocator.LocatorBound>) {
        self.locator = locator
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohLocator {
        var reader = cursor
        let locator_len = try reader.readVle(UInt64.self)
        if locator_len > 255 {
            throw CodecError.lengthOutOfRange
        }
        let locator = try BoundedText<CodecZenohLocator.LocatorBound>(decoding: try reader.readString(Int(clamping: locator_len)))
        cursor = reader
        return CodecZenohLocator(locator: locator)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohLocatorSource` is written,
    /// with no copy into `CodecZenohLocator`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohLocatorSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let locator_len_derived: UInt64 = UInt64(source.locator.byteCount)
        var frame: [UInt8] = []
        appendVle(locator_len_derived, to: &frame)
        source.locator.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
