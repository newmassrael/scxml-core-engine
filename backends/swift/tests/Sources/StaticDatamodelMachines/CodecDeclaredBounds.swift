// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_declared_bounds.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecDeclaredBounds` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecDeclaredBounds.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecDeclaredBounds`, which conforms itself.
public protocol CodecDeclaredBoundsSource {
    /// What each entry of `items` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Items_Entry: CodecRepeatElemSource
    var key: BoundedBytes<CodecDeclaredBounds.KeyBound> { get }
    var tag: BoundedBytes<CodecDeclaredBounds.TagBound> { get }
    var items: BoundedList<Items_Entry, CodecDeclaredBounds.ItemsBound> { get }
}

/// The frame this codec lays on the wire: 6 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBounds: Equatable, Sendable, CodecDeclaredBoundsSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3

    /// How many bytes `key` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum KeyBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 8
    }

    /// How many bytes `tag` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum TagBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 4
    }

    /// How many entries `items` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ItemsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 2
    }

    /// The value presents each entry of `items` as a value of its own codec.
    public typealias Items_Entry = CodecRepeatElem

    /// The bytes, as many as `key_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var key: BoundedBytes<CodecDeclaredBounds.KeyBound> = .fewest

    /// The bytes, as many as `tag_len` says, plus -1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var tag: BoundedBytes<CodecDeclaredBounds.TagBound> = .fewest

    /// As many entries as `count` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var items: BoundedList<CodecRepeatElem, CodecDeclaredBounds.ItemsBound> = .fewest(of: CodecRepeatElem())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(key: BoundedBytes<CodecDeclaredBounds.KeyBound>, tag: BoundedBytes<CodecDeclaredBounds.TagBound>, items: BoundedList<Items_Entry, CodecDeclaredBounds.ItemsBound>) {
        self.key = key
        self.tag = tag
        self.items = items
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecDeclaredBounds {
        var reader = cursor
        let key_len = try reader.readVle(UInt64.self)
        if key_len > 8 {
            throw CodecError.lengthOutOfRange
        }
        let key = try BoundedBytes<CodecDeclaredBounds.KeyBound>(decoding: Array(try reader.readSlice(Int(clamping: key_len))))
        let tag_len = try reader.readInteger(UInt8.self, .big)
        if tag_len < 1 || tag_len > 5 {
            throw CodecError.lengthOutOfRange
        }
        let tag = try BoundedBytes<CodecDeclaredBounds.TagBound>(decoding: Array(try reader.readSlice(Int(clamping: tag_len) &+ (-1))))
        let count = try reader.readInteger(UInt8.self, .big)
        if count > 2 {
            throw CodecError.tooManyElements
        }
        var items_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: count) {
            items_items.append(try CodecRepeatElem.decode(&reader))
        }
        let items = try BoundedList<CodecRepeatElem, CodecDeclaredBounds.ItemsBound>(decoding: items_items)
        cursor = reader
        return CodecDeclaredBounds(key: key, tag: tag, items: items)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecDeclaredBoundsSource` is written,
    /// with no copy into `CodecDeclaredBounds`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecDeclaredBoundsSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let key_len_derived: UInt64 = UInt64(source.key.byteCount)
        let tag_len_derived: UInt8 = UInt8((source.tag.byteCount + 1))
        let count_derived: UInt8 = UInt8(source.items.count)
        var frame: [UInt8] = []
        appendVle(key_len_derived, to: &frame)
        source.key.append(to: &frame)
        appendInteger(tag_len_derived, to: &frame, .big)
        source.tag.append(to: &frame)
        appendInteger(count_derived, to: &frame, .big)
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
