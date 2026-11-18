// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_source_parent_tag.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecSourceParentTag` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecSourceParentTag.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecSourceParentTag`, which conforms itself.
public protocol CodecSourceParentTagSource {
    /// What `body` is presented as: a source of `CodecSourceLeaf`, written by its encode.
    associatedtype Body_Source: CodecSourceLeafSource
    var hdr: UInt8 { get }
    var body: Body_Source { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSourceParentTag: Equatable, Sendable, CodecSourceParentTagSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecSourceLeaf.frameLength

    /// The value presents `body` as a value of its own codec.
    public typealias Body_Source = CodecSourceLeaf

    /// The field at byte 0, big-endian.
    public var hdr: UInt8 = 0

    /// Bit 0 of `hdr` (flag `kind`).
    public var kind: Bool {
        get { (hdr & 0x1) != 0 }
        set { hdr = newValue ? (hdr | 0x1) : (hdr & ~0x1) }
    }

    /// Bit 7 of `hdr` (flag `more`).
    public var more: Bool {
        get { (hdr & 0x80) != 0 }
        set { hdr = newValue ? (hdr | 0x80) : (hdr & ~0x80) }
    }

    /// A `CodecSourceLeaf` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var body: CodecSourceLeaf = CodecSourceLeaf()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(hdr: UInt8, body: Body_Source) {
        self.hdr = hdr
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSourceParentTag {
        var reader = cursor
        let hdr = try reader.readInteger(UInt8.self, .big)
        let body = try CodecSourceLeaf.decode(&reader, tag: UInt8(truncatingIfNeeded: (hdr >> 0) & 0x1))
        cursor = reader
        return CodecSourceParentTag(hdr: hdr, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecSourceParentTag` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `more` flag of `hdr`, where a parent sets it.
        public var hdrMore: Bool?

        public init(hdrMore: Bool? = nil) {
            self.hdrMore = hdrMore
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecSourceParentTagSource` is written,
    /// with no copy into `CodecSourceParentTag`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecSourceParentTagSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var hdr_kind_filled: UInt64 = 0
        hdr_kind_filled |= UInt64(source.body.body.tag) & 0x1
        var hdr_folded = source.hdr
        hdr_folded = (hdr_folded & ~0x1) | ((UInt8(truncatingIfNeeded: hdr_kind_filled) & 0x1) << 0)
        if let set = overrides.hdrMore { hdr_folded = set ? (hdr_folded | 0x80) : (hdr_folded & ~0x80) }
        var body_piece = BytesSink()
        try CodecSourceLeaf.encodeSource(source.body, into: &body_piece)
        let body_bytes = body_piece.bytes
        var frame: [UInt8] = []
        appendInteger(hdr_folded, to: &frame, .big)
        frame += body_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
