// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_repeat_flag_aligned_group.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatFlagAlignedGroup: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var head: UInt8 = 0

    /// Bit 0 of `head` (flag `W`).
    public var W: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// Bit 1 of `head` (flag `H`).
    public var H: Bool {
        get { (head & 0x2) != 0 }
        set { head = newValue ? (head | 0x2) : (head & ~0x2) }
    }

    /// The field at byte 1, big-endian.
    public var links_len: UInt8 = 0

    /// As many entries as `links_len` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var links: [CodecRepeatElem] = []

    /// On the wire only when `(head & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `links_len` says, each a `CodecChainFlagEntry`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var weights: [CodecChainFlagEntry]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, links_len: UInt8, links: [CodecRepeatElem], weights: [CodecChainFlagEntry]?) {
        self.head = head
        self.links_len = links_len
        self.links = links
        self.weights = weights
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatFlagAlignedGroup {
        var reader = cursor
        var frame = CodecRepeatFlagAlignedGroup()
        frame.head = try reader.readInteger(UInt8.self, .big)
        frame.links_len = try reader.readInteger(UInt8.self, .big)
        if frame.links_len > 4 {
            throw CodecError.tooManyElements
        }
        var links_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: frame.links_len) {
            links_items.append(try CodecRepeatElem.decode(&reader))
        }
        frame.links = links_items
        if (frame.head & 0x2) != 0 {
            if frame.links_len > 4 {
                throw CodecError.tooManyElements
            }
            var weights_items: [CodecChainFlagEntry] = []
            for _ in 0..<Int(clamping: frame.links_len) {
                weights_items.append(try CodecChainFlagEntry.decode(&reader, wide: UInt8(truncatingIfNeeded: (frame.head >> 0) & 0x1)))
            }
            frame.weights = weights_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.links.count > 4 {
            throw CodecError.tooManyElements
        }
        if let value = self.weights, value.count > 4 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.head, to: &frame, .big)
        appendInteger(self.links_len, to: &frame, .big)
        for entry in self.links {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        if let weights = self.weights {
            for entry in weights {
                var piece = BytesSink()
                try entry.encode(into: &piece, wide: UInt8(truncatingIfNeeded: (self.head >> 0) & 0x1))
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
