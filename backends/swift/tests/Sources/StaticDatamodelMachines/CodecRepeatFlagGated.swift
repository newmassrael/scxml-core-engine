// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_repeat_flag_gated.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatFlagGated: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var head: UInt8 = 0

    /// Bit 0 of `head` (flag `W`).
    public var W: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// Bit 1 of `head` (flag `L`).
    public var L: Bool {
        get { (head & 0x2) != 0 }
        set { head = newValue ? (head | 0x2) : (head & ~0x2) }
    }

    /// On the wire only when `(head & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var count: UInt8? = nil

    /// On the wire only when `(head & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `count` says, each a `CodecChainFlagEntry`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var entries: [CodecChainFlagEntry]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, count: UInt8?, entries: [CodecChainFlagEntry]?) {
        self.head = head
        self.count = count
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatFlagGated {
        var reader = cursor
        var frame = CodecRepeatFlagGated()
        frame.head = try reader.readInteger(UInt8.self, .big)
        if (frame.head & 0x2) != 0 {
            frame.count = try reader.readInteger(UInt8.self, .big)
        }
        if (frame.head & 0x2) != 0 {
            if (frame.count ?? 0) > 4 {
                throw CodecError.tooManyElements
            }
            var entries_items: [CodecChainFlagEntry] = []
            for _ in 0..<Int(clamping: (frame.count ?? 0)) {
                entries_items.append(try CodecChainFlagEntry.decode(&reader, wide: UInt8(truncatingIfNeeded: (frame.head >> 0) & 0x1)))
            }
            frame.entries = entries_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.entries, value.count > 4 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.head, to: &frame, .big)
        if let count = self.count {
            appendInteger(count, to: &frame, .big)
        }
        if let entries = self.entries {
            for entry in entries {
                var piece = BytesSink()
                try entry.encode(into: &piece, wide: UInt8(truncatingIfNeeded: (self.head >> 0) & 0x1))
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
