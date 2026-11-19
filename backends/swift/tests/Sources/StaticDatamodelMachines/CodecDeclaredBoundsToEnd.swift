// SCE-GENERATED — DO NOT EDIT
// source-hash: f7b651ac1dcd2e75941129242bb3d51998ea8be14ccd62e40f4f923e16c957e4
// source: codec_declared_bounds_to_end.scxml
// SCE-MAP: codec_declared_bounds_to_end.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecDeclaredBoundsToEnd` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecDeclaredBoundsToEnd.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecDeclaredBoundsToEnd`, which conforms itself.
public protocol CodecDeclaredBoundsToEndSource {
    /// What each entry of `items` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Items_Entry: CodecRepeatElemSource
    var items: BoundedList<Items_Entry, CodecDeclaredBoundsToEnd.ItemsBound> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBoundsToEnd: Equatable, Sendable, CodecDeclaredBoundsToEndSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// How many entries `items` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ItemsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 2
    }

    /// The value presents each entry of `items` as a value of its own codec.
    public typealias Items_Entry = CodecRepeatElem

    /// The entries to the end of the frame, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame with more is refused with `tooManyElements`, and a value with
    /// more is no value of its type.
    public var items: BoundedList<CodecRepeatElem, CodecDeclaredBoundsToEnd.ItemsBound> = .fewest(of: CodecRepeatElem())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(items: BoundedList<Items_Entry, CodecDeclaredBoundsToEnd.ItemsBound>) {
        self.items = items
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecDeclaredBoundsToEnd {
        var reader = cursor
        var items_items: [CodecRepeatElem] = []
        while reader.remaining > 0 {
            if items_items.count > 1 {
                throw CodecError.tooManyElements
            }
            items_items.append(try CodecRepeatElem.decode(&reader))
        }
        let items = try BoundedList<CodecRepeatElem, CodecDeclaredBoundsToEnd.ItemsBound>(decoding: items_items)
        cursor = reader
        return CodecDeclaredBoundsToEnd(items: items)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecDeclaredBoundsToEndSource` is written,
    /// with no copy into `CodecDeclaredBoundsToEnd`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecDeclaredBoundsToEndSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var items_entries = BytesSink()
        for entry in source.items {
            try CodecRepeatElem.encodeSource(entry, into: &items_entries)
        }
        frame += items_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
