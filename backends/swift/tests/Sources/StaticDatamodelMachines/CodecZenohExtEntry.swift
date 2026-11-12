// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_zenoh_ext_entry.scxml:52 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohExtEntry` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecZenohExtEntryVariant: Equatable, Sendable {
    case codecZenohExtUnit(CodecZenohExtUnit)
    case codecZenohExtZint(CodecZenohExtZint)
    case codecZenohExtZbuf(CodecZenohExtZbuf)
    case `default`(tag: UInt8, body: CodecZenohExtUnit)
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohExtEntry: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecZenohExtUnit.frameLength, CodecZenohExtZint.frameLength, CodecZenohExtZbuf.frameLength)

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 0 (flag `ext_id`). A value
    /// wider than the flag is cut to it when written.
    public var extId: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0xF) }
        set { header = (header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 4 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x10) != 0 }
        set { header = newValue ? (header | 0x10) : (header & ~0x10) }
    }

    /// The 2 bits of `header` from bit 5 (flag `enc`). A value
    /// wider than the flag is cut to it when written.
    public var enc: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 5) & 0x3) }
        set { header = (header & ~(0x3 << 5)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 5) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohExtEntryVariant = .codecZenohExtUnit(CodecZenohExtUnit())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecZenohExtEntryVariant) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohExtEntry {
        var reader = cursor
        var frame = CodecZenohExtEntry()
        frame.header = try reader.readInteger(UInt8.self, .big)
        let body_tag = (frame.header >> 5) & 0x3
        switch body_tag {
        case 0x0:
            frame.body = .codecZenohExtUnit(try CodecZenohExtUnit.decode(&reader))
        case 0x1:
            frame.body = .codecZenohExtZint(try CodecZenohExtZint.decode(&reader))
        case 0x2:
            frame.body = .codecZenohExtZbuf(try CodecZenohExtZbuf.decode(&reader))
        default:
            frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohExtUnit.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecZenohExtUnit(let arm):
                try arm.encode(into: &piece)
            case .codecZenohExtZint(let arm):
                try arm.encode(into: &piece)
            case .codecZenohExtZbuf(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
