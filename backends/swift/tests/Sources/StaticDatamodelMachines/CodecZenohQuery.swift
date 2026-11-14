// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: codec_zenoh_query.scxml:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohQuery: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x3

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `C`).
    public var C: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `P`).
    public var P: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var consolidation: UInt8? = nil

    /// On the wire only when `(header & 0x40) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var parameters_len: UInt64? = nil

    /// On the wire only when `(header & 0x40) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `parameters_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var parameters: [UInt8]? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: to the end of the frame. The protocol states no depth.
    public var extensions: [CodecZenohExtEntry]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, consolidation: UInt8?, parameters_len: UInt64?, parameters: [UInt8]?, extensions: [CodecZenohExtEntry]?) {
        self.header = header
        self.consolidation = consolidation
        self.parameters_len = parameters_len
        self.parameters = parameters
        self.extensions = extensions
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohQuery {
        var reader = cursor
        var frame = CodecZenohQuery()
        frame.header = try reader.readInteger(UInt8.self, .big)
        if (frame.header & 0x20) != 0 {
            frame.consolidation = try reader.readInteger(UInt8.self, .big)
        }
        if (frame.header & 0x40) != 0 {
            frame.parameters_len = try reader.readVle(UInt64.self)
        }
        if (frame.header & 0x40) != 0 {
            frame.parameters = Array(try reader.readSlice(Int(clamping: (frame.parameters_len ?? 0))))
        }
        if (frame.header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            while reader.remaining > 0 {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_items.append(entry)
            }
            frame.extensions = extensions_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        if let consolidation = self.consolidation {
            appendInteger(consolidation, to: &frame, .big)
        }
        if let parameters_len = self.parameters_len {
            appendVle(parameters_len, to: &frame)
        }
        if let parameters = self.parameters {
            frame += parameters
        }
        if let extensions = self.extensions {
            for entry in extensions {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
