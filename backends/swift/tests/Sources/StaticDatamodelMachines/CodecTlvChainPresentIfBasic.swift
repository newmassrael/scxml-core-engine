// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_tlv_chain_present_if_basic.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecTlvChainPresentIfBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// Bit 0 of `carrier` (flag `has_chain`).
    public var hasChain: Bool {
        get { (carrier & 0x1) != 0 }
        set { carrier = newValue ? (carrier | 0x1) : (carrier & ~0x1) }
    }

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecTlvEntry`, one after another, at most 4: to the end of the frame. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var entries: [CodecTlvEntry]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, entries: [CodecTlvEntry]?) {
        self.carrier = carrier
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecTlvChainPresentIfBasic {
        var reader = cursor
        var frame = CodecTlvChainPresentIfBasic()
        frame.carrier = try reader.readInteger(UInt8.self, .big)
        if (frame.carrier & 0x1) != 0 {
            var entries_items: [CodecTlvEntry] = []
            for _ in 0..<4 {
                if reader.remaining == 0 { break }
                let entry = try CodecTlvEntry.decode(&reader)
                entries_items.append(entry)
            }
            if reader.remaining > 0 {
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
        if let value = self.entries, value.count > 4 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.carrier, to: &frame, .big)
        if let entries = self.entries {
            for entry in entries {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
