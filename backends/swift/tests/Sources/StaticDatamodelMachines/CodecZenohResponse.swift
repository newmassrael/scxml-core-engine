// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: codec_zenoh_response.scxml:76 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohResponse` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecZenohResponseVariant: Equatable, Sendable {
    case codecZenohReply(CodecZenohReply)
    case codecZenohErr(CodecZenohErr)
    case `default`(tag: UInt8, body: CodecZenohReply)
}

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohResponse: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3 + Swift.min(CodecZenohReply.frameLength, CodecZenohErr.frameLength)

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1B

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
    public var request_id: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var key_id: UInt32 = 0

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var suffix_len: UInt64? = nil

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `suffix_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var suffix: String? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: [CodecZenohExtEntry]? = nil

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohResponseVariant = .codecZenohReply(CodecZenohReply())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, request_id: UInt64, key_id: UInt32, suffix_len: UInt64?, suffix: String?, extensions: [CodecZenohExtEntry]?, body: CodecZenohResponseVariant) {
        self.header = header
        self.request_id = request_id
        self.key_id = key_id
        self.suffix_len = suffix_len
        self.suffix = suffix
        self.extensions = extensions
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohResponse {
        var reader = cursor
        var frame = CodecZenohResponse()
        frame.header = try reader.readInteger(UInt8.self, .big)
        frame.request_id = try reader.readVle(UInt64.self)
        frame.key_id = try reader.readVle(UInt32.self)
        if (frame.header & 0x20) != 0 {
            frame.suffix_len = try reader.readVle(UInt64.self)
        }
        if (frame.header & 0x20) != 0 {
            if (frame.suffix_len ?? 0) > 65535 {
                throw CodecError.lengthOutOfRange
            }
            frame.suffix = try reader.readString(Int(clamping: (frame.suffix_len ?? 0)))
        }
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
        case 0x4:
            frame.body = .codecZenohReply(try CodecZenohReply.decode(&reader))
        case 0x5:
            frame.body = .codecZenohErr(try CodecZenohErr.decode(&reader))
        default:
            frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohReply.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.suffix, value.utf8.count > 65535 {
            throw CodecError.lengthOutOfRange
        }
        if let value = self.extensions, value.count < 1 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        appendVle(self.request_id, to: &frame)
        appendVle(self.key_id, to: &frame)
        if let suffix_len = self.suffix_len {
            appendVle(suffix_len, to: &frame)
        }
        if let suffix = self.suffix {
            frame += Array(suffix.utf8)
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
            case .codecZenohReply(let arm):
                try arm.encode(into: &piece)
            case .codecZenohErr(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
