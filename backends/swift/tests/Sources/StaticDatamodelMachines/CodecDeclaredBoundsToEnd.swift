// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: codec_declared_bounds_to_end.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBoundsToEnd: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// The entries to the end of the frame, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame with more is refused with `tooManyElements`, as is a value with
    /// more at its encode.
    public var items: [CodecRepeatElem] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(items: [CodecRepeatElem]) {
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
        var frame = CodecDeclaredBoundsToEnd()
        var items_items: [CodecRepeatElem] = []
        while reader.remaining > 0 {
            if items_items.count > 1 {
                throw CodecError.tooManyElements
            }
            items_items.append(try CodecRepeatElem.decode(&reader))
        }
        frame.items = items_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.items.count > 2 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        for entry in self.items {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
