// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: codec_chain_has_tagged.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasTagged: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 0 of `header` (flag `wide`).
    public var wide: Bool {
        get { (header & 0x1) != 0 }
        set { header = newValue ? (header | 0x1) : (header & ~0x1) }
    }

    /// Entries of `CodecChainHasTaggedEntry`, one after another, at most 3: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var entries: [CodecChainHasTaggedEntry] = []

    /// On the wire only when `_has_entries_7 || (header & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 2, big-endian.
    public var priority: UInt8? = nil

    /// On the wire only when `!_has_entries_9`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 3, big-endian.
    public var checksum: UInt16? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, entries: [CodecChainHasTaggedEntry], priority: UInt8?, checksum: UInt16?) {
        self.header = header
        self.entries = entries
        self.priority = priority
        self.checksum = checksum
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasTagged {
        var reader = cursor
        var frame = CodecChainHasTagged()
        frame.header = try reader.readInteger(UInt8.self, .big)
        var entries_items: [CodecChainHasTaggedEntry] = []
        var entries_more = false
        for _ in 0..<3 {
            let entry = try CodecChainHasTaggedEntry.decode(&reader)
            entries_more = entry.more
            entries_items.append(entry)
            if !entries_more { break }
        }
        if entries_more && reader.remaining == 0 {
            throw CodecError.needMoreBytes
        }
        if entries_more {
            throw CodecError.tooManyElements
        }
        frame.entries = entries_items
        let _has_entries_7 = frame.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })
        let _has_entries_9 = frame.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 9 })
        if _has_entries_7 || (frame.header & 0x1) != 0 {
            frame.priority = try reader.readInteger(UInt8.self, .big)
        }
        if !_has_entries_9 {
            frame.checksum = try reader.readInteger(UInt16.self, .big)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        let _has_entries_7 = self.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })
        let _has_entries_9 = self.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 9 })
        if _has_entries_7 || (self.header & 0x1) != 0 {
            if self.priority == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.priority != nil {
            throw CodecError.presentIfMismatch
        }
        if !_has_entries_9 {
            if self.checksum == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.checksum != nil {
            throw CodecError.presentIfMismatch
        }
        if self.entries.count < 1 || self.entries.count > 3 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        for entry in self.entries {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        if let priority = self.priority {
            appendInteger(priority, to: &frame, .big)
        }
        if let checksum = self.checksum {
            appendInteger(checksum, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }
}
