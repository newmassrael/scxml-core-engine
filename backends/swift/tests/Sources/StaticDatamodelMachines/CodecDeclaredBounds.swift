// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_declared_bounds.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 6 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBounds: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var key_len: UInt64 = 0

    /// The bytes, as many as `key_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var key: [UInt8] = []

    /// The field at byte 2, big-endian.
    public var tag_len: UInt8 = 0

    /// The bytes, as many as `tag_len` says, plus -1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var tag: [UInt8] = []

    /// The field at byte 4, big-endian.
    public var count: UInt8 = 0

    /// As many entries as `count` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var items: [CodecRepeatElem] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(key_len: UInt64, key: [UInt8], tag_len: UInt8, tag: [UInt8], count: UInt8, items: [CodecRepeatElem]) {
        self.key_len = key_len
        self.key = key
        self.tag_len = tag_len
        self.tag = tag
        self.count = count
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
        var frame = CodecDeclaredBounds()
        frame.key_len = try reader.readVle(UInt64.self)
        if frame.key_len > 8 {
            throw CodecError.lengthOutOfRange
        }
        frame.key = Array(try reader.readSlice(Int(clamping: frame.key_len)))
        frame.tag_len = try reader.readInteger(UInt8.self, .big)
        if frame.tag_len < 1 || frame.tag_len > 5 {
            throw CodecError.lengthOutOfRange
        }
        frame.tag = Array(try reader.readSlice(Int(clamping: frame.tag_len) &+ (-1)))
        frame.count = try reader.readInteger(UInt8.self, .big)
        if frame.count > 2 {
            throw CodecError.tooManyElements
        }
        var items_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: frame.count) {
            items_items.append(try CodecRepeatElem.decode(&reader))
        }
        frame.items = items_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.key.count > 8 {
            throw CodecError.lengthOutOfRange
        }
        if self.tag.count > 4 {
            throw CodecError.lengthOutOfRange
        }
        if self.items.count > 2 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendVle(self.key_len, to: &frame)
        frame += self.key
        appendInteger(self.tag_len, to: &frame, .big)
        frame += self.tag
        appendInteger(self.count, to: &frame, .big)
        for entry in self.items {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
