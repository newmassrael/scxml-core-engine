// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_chain_prev_gated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainPrevGated: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var head: UInt8 = 0

    /// Bit 0 of `head` (flag `T`).
    public var T: Bool {
        get { (head & 0x1) != 0 }
        set { head = newValue ? (head | 0x1) : (head & ~0x1) }
    }

    /// Bit 1 of `head` (flag `E`).
    public var E: Bool {
        get { (head & 0x2) != 0 }
        set { head = newValue ? (head | 0x2) : (head & ~0x2) }
    }

    /// On the wire only when `(head & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecChainPrevEntry`, one after another, at most 4: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var entries: [CodecChainPrevEntry]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(head: UInt8, entries: [CodecChainPrevEntry]?) {
        self.head = head
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainPrevGated {
        var reader = cursor
        var frame = CodecChainPrevGated()
        frame.head = try reader.readInteger(UInt8.self, .big)
        if (frame.head & 0x2) != 0 {
            var _prev_entries_after_marker: UInt8 = 0
            var entries_items: [CodecChainPrevEntry] = []
            var entries_more = false
            for _ in 0..<4 {
                let entry = try CodecChainPrevEntry.decode(&reader, after_marker: _prev_entries_after_marker, tagged: UInt8(truncatingIfNeeded: (frame.head >> 0) & 0x1))
                entries_more = entry.Z
                entries_items.append(entry)
                _prev_entries_after_marker = ((chainIdentifier(entry.header) & 127) == 4) ? 1 : 0
                if !entries_more { break }
            }
            if entries_more && reader.remaining == 0 {
                throw CodecError.needMoreBytes
            }
            if entries_more {
                throw CodecError.tooManyElements
            }
            frame.entries = entries_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.entries, value.count < 1 || value.count > 4 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.head, to: &frame, .big)
        if let entries = self.entries {
            var _prev_entries_after_marker: UInt8 = 0
            for entry in entries {
                var piece = BytesSink()
                try entry.encode(into: &piece, after_marker: _prev_entries_after_marker, tagged: UInt8(truncatingIfNeeded: (self.head >> 0) & 0x1))
                frame += piece.bytes
                _prev_entries_after_marker = ((chainIdentifier(entry.header) & 127) == 4) ? 1 : 0
            }
        }
        try sink.writeBytes(frame)
    }
}
