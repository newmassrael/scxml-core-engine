// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_zenoh_request.scxml:73 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohRequest` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecZenohRequestVariant: Equatable, Sendable {
    case codecZenohMsgPut(CodecZenohMsgPut)
    case codecZenohMsgDel(CodecZenohMsgDel)
    case codecZenohQuery(CodecZenohQuery)
    case `default`(tag: UInt8, body: CodecZenohQuery)
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohRequest: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecZenohWireexpr.frameLength + Swift.min(CodecZenohMsgPut.frameLength, CodecZenohMsgDel.frameLength, CodecZenohQuery.frameLength)

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1C

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `N`).
    public var N: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var rid: UInt64 = 0

    /// A `CodecZenohWireexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var keyexpr: CodecZenohWireexpr = CodecZenohWireexpr()

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: [CodecZenohExtEntry]? = nil

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohRequestVariant = .codecZenohMsgPut(CodecZenohMsgPut())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, rid: UInt64, keyexpr: CodecZenohWireexpr, extensions: [CodecZenohExtEntry]?, body: CodecZenohRequestVariant) {
        self.header = header
        self.rid = rid
        self.keyexpr = keyexpr
        self.extensions = extensions
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohRequest {
        var reader = cursor
        var frame = CodecZenohRequest()
        frame.header = try reader.readInteger(UInt8.self, .big)
        frame.rid = try reader.readVle(UInt64.self)
        frame.keyexpr = try CodecZenohWireexpr.decode(&reader, N: UInt8(truncatingIfNeeded: (frame.header >> 5) & 0x1))
        if (frame.header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            frame.extensions = extensions_items
        }
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1F
        switch body_tag {
        case 0x1:
            frame.body = .codecZenohMsgPut(try CodecZenohMsgPut.decode(&reader))
        case 0x2:
            frame.body = .codecZenohMsgDel(try CodecZenohMsgDel.decode(&reader))
        case 0x3:
            frame.body = .codecZenohQuery(try CodecZenohQuery.decode(&reader))
        default:
            frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohQuery.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.extensions, value.count < 1 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        appendVle(self.rid, to: &frame)
        do {
            var piece = BytesSink()
            try self.keyexpr.encode(into: &piece, N: UInt8(truncatingIfNeeded: (self.header >> 5) & 0x1))
            frame += piece.bytes
        }
        if let extensions = self.extensions {
            for entry in extensions {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecZenohMsgPut(let arm):
                try arm.encode(into: &piece)
            case .codecZenohMsgDel(let arm):
                try arm.encode(into: &piece)
            case .codecZenohQuery(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
