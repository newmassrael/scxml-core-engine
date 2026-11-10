// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_until_eof_basic.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecUntilEofBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// The entries to the end of the frame, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame with more is refused with `tooManyElements`, as is a value with
    /// more at its encode.
    public var msgs: [CodecRepeatElem] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(msgs: [CodecRepeatElem]) {
        self.msgs = msgs
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecUntilEofBasic {
        var reader = cursor
        var frame = CodecUntilEofBasic()
        var msgs_items: [CodecRepeatElem] = []
        while reader.remaining > 0 {
            if msgs_items.count > 63 {
                throw CodecError.tooManyElements
            }
            msgs_items.append(try CodecRepeatElem.decode(&reader))
        }
        frame.msgs = msgs_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.msgs.count > 64 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        for entry in self.msgs {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
