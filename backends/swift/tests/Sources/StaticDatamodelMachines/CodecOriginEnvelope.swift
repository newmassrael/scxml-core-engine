// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_origin_envelope.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOriginEnvelope: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3 + CodecOriginLeaf.frameLength

    /// The field at byte 0, big-endian.
    public var hdr: UInt8 = 0

    /// Bit 0 of `hdr` (flag `has_opt`).
    public var hasOpt: Bool {
        get { (hdr & 0x1) != 0 }
        set { hdr = newValue ? (hdr | 0x1) : (hdr & ~0x1) }
    }

    /// The field at byte 1, big-endian.
    public var note_len: UInt8 = 0

    /// The text of as many UTF-8 bytes as `note_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var note: String = ""

    /// The field at byte 3, big-endian.
    public var m: UInt8 = 0

    /// A `CodecOriginLeaf` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var required: CodecOriginLeaf = CodecOriginLeaf()

    /// On the wire only when `(hdr & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecOriginLeaf` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var optional: CodecOriginLeaf? = nil

    /// As many entries as `m` says, each a `CodecOriginLeaf`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var items: [CodecOriginLeaf] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(hdr: UInt8, note_len: UInt8, note: String, m: UInt8, required: CodecOriginLeaf, optional: CodecOriginLeaf?, items: [CodecOriginLeaf]) {
        self.hdr = hdr
        self.note_len = note_len
        self.note = note
        self.m = m
        self.required = required
        self.optional = optional
        self.items = items
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOriginEnvelope {
        var reader = cursor
        var frame = CodecOriginEnvelope()
        frame.hdr = try reader.readInteger(UInt8.self, .big)
        frame.note_len = try reader.readInteger(UInt8.self, .big)
        if frame.note_len > 16 {
            throw CodecError.lengthOutOfRange
        }
        frame.note = try reader.readString(Int(clamping: frame.note_len))
        frame.m = try reader.readInteger(UInt8.self, .big)
        frame.required = try CodecOriginLeaf.decode(&reader)
        if (frame.hdr & 0x1) != 0 {
            frame.optional = try CodecOriginLeaf.decode(&reader)
        }
        if frame.m > 3 {
            throw CodecError.tooManyElements
        }
        var items_items: [CodecOriginLeaf] = []
        for _ in 0..<Int(clamping: frame.m) {
            items_items.append(try CodecOriginLeaf.decode(&reader))
        }
        frame.items = items_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.note.utf8.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        if self.items.count > 3 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.hdr, to: &frame, .big)
        appendInteger(self.note_len, to: &frame, .big)
        frame += Array(self.note.utf8)
        appendInteger(self.m, to: &frame, .big)
        do {
            var piece = BytesSink()
            try self.required.encode(into: &piece)
            frame += piece.bytes
        }
        if let optional = self.optional {
            do {
                var piece = BytesSink()
                try optional.encode(into: &piece)
                frame += piece.bytes
            }
        }
        for entry in self.items {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
