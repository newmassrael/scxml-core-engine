// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_repeat_basic.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var num_frags: UInt8 = 0

    /// As many entries as `num_frags` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var frags: [CodecRepeatElem] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(num_frags: UInt8, frags: [CodecRepeatElem]) {
        self.num_frags = num_frags
        self.frags = frags
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatBasic {
        var reader = cursor
        var frame = CodecRepeatBasic()
        frame.num_frags = try reader.readInteger(UInt8.self, .big)
        if frame.num_frags > 32 {
            throw CodecError.tooManyElements
        }
        var frags_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: frame.num_frags) {
            frags_items.append(try CodecRepeatElem.decode(&reader))
        }
        frame.frags = frags_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.frags.count > 32 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.num_frags, to: &frame, .big)
        for entry in self.frags {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
