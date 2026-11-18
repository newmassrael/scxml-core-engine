// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_origin_envelope.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecOriginEnvelope` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecOriginEnvelope.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecOriginEnvelope`, which conforms itself.
public protocol CodecOriginEnvelopeSource {
    /// What `required` is presented as: a source of `CodecOriginLeaf`, written by its encode.
    associatedtype Required_Source: CodecOriginLeafSource
    /// What `optional` is presented as: a source of `CodecOriginLeaf`, written by its encode.
    associatedtype Optional_Source: CodecOriginLeafSource
    /// What each entry of `items` is presented as: a source of `CodecOriginLeaf`, written by its encode.
    associatedtype Items_Entry: CodecOriginLeafSource
    var hdr: UInt8 { get }
    var note: BoundedText<CodecOriginEnvelope.NoteBound> { get }
    var required: Required_Source { get }
    var optional: Optional_Source? { get }
    var items: BoundedList<Items_Entry, CodecOriginEnvelope.ItemsBound> { get }
}

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOriginEnvelope: Equatable, Sendable, CodecOriginEnvelopeSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3 + CodecOriginLeaf.frameLength

    /// How many bytes `note` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum NoteBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 16
    }

    /// How many entries `items` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ItemsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 3
    }

    /// The value presents `required` as a value of its own codec.
    public typealias Required_Source = CodecOriginLeaf

    /// The value presents `optional` as a value of its own codec.
    public typealias Optional_Source = CodecOriginLeaf

    /// The value presents each entry of `items` as a value of its own codec.
    public typealias Items_Entry = CodecOriginLeaf

    /// The field at byte 0, big-endian.
    public var hdr: UInt8 = 0

    /// Bit 0 of `hdr` (flag `has_opt`).
    public var hasOpt: Bool {
        get { (hdr & 0x1) != 0 }
        set { hdr = newValue ? (hdr | 0x1) : (hdr & ~0x1) }
    }

    /// The text of as many UTF-8 bytes as `note_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var note: BoundedText<CodecOriginEnvelope.NoteBound> = .fewest

    /// A `CodecOriginLeaf` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var required: CodecOriginLeaf = CodecOriginLeaf()

    /// On the wire only when `(hdr & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecOriginLeaf` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var optional: CodecOriginLeaf? = nil

    /// As many entries as `m` says, each a `CodecOriginLeaf`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var items: BoundedList<CodecOriginLeaf, CodecOriginEnvelope.ItemsBound> = .fewest(of: CodecOriginLeaf())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(hdr: UInt8, note: BoundedText<CodecOriginEnvelope.NoteBound>, required: Required_Source, optional: Optional_Source?, items: BoundedList<Items_Entry, CodecOriginEnvelope.ItemsBound>) {
        self.hdr = hdr
        self.note = note
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
        let hdr = try reader.readInteger(UInt8.self, .big)
        let note_len = try reader.readInteger(UInt8.self, .big)
        if note_len > 16 {
            throw CodecError.lengthOutOfRange
        }
        let note = try BoundedText<CodecOriginEnvelope.NoteBound>(decoding: try reader.readString(Int(clamping: note_len)))
        let m = try reader.readInteger(UInt8.self, .big)
        let required = try CodecOriginLeaf.decode(&reader)
        var optional: CodecOriginLeaf? = nil
        if (hdr & 0x1) != 0 {
            optional = try CodecOriginLeaf.decode(&reader)
        }
        if m > 3 {
            throw CodecError.tooManyElements
        }
        var items_items: [CodecOriginLeaf] = []
        for _ in 0..<Int(clamping: m) {
            items_items.append(try CodecOriginLeaf.decode(&reader))
        }
        let items = try BoundedList<CodecOriginLeaf, CodecOriginEnvelope.ItemsBound>(decoding: items_items)
        cursor = reader
        return CodecOriginEnvelope(hdr: hdr, note: note, required: required, optional: optional, items: items)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecOriginEnvelopeSource` is written,
    /// with no copy into `CodecOriginEnvelope`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecOriginEnvelopeSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let note_len_derived: UInt8 = UInt8(source.note.byteCount)
        let m_derived: UInt8 = UInt8(source.items.count)
        var hdr_folded = source.hdr
        hdr_folded = (hdr_folded & ~0x1) | ((source.optional != nil) ? 0x1 : 0)
        var required_piece = BytesSink()
        try CodecOriginLeaf.encodeSource(source.required, into: &required_piece)
        let required_bytes = required_piece.bytes
        var optional_bytes: [UInt8]? = nil
        if let held = source.optional {
            var piece = BytesSink()
            try CodecOriginLeaf.encodeSource(held, into: &piece)
            optional_bytes = piece.bytes
        }
        var frame: [UInt8] = []
        appendInteger(hdr_folded, to: &frame, .big)
        appendInteger(note_len_derived, to: &frame, .big)
        source.note.append(to: &frame)
        appendInteger(m_derived, to: &frame, .big)
        frame += required_bytes
        if let held = optional_bytes { frame += held }
        var items_entries = BytesSink()
        for entry in source.items {
            try CodecOriginLeaf.encodeSource(entry, into: &items_entries)
        }
        frame += items_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
