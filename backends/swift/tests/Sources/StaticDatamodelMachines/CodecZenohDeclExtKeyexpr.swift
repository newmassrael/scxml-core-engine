// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_decl_ext_keyexpr.scxml:89 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohDeclExtKeyexpr` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohDeclExtKeyexpr.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohDeclExtKeyexpr`, which conforms itself.
public protocol CodecZenohDeclExtKeyexprSource {
    /// What `inner` is presented as: a source of `CodecZenohDeclExtKeyexprInner`, written by its encode.
    associatedtype Inner_Source: CodecZenohDeclExtKeyexprInnerSource
    var outer_header: UInt8 { get }
    var inner: Inner_Source { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclExtKeyexpr: Equatable, Sendable, CodecZenohDeclExtKeyexprSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecZenohDeclExtKeyexprInner.frameLength

    /// The value presents `inner` as a value of its own codec.
    public typealias Inner_Source = CodecZenohDeclExtKeyexprInner

    /// The field at byte 0, big-endian.
    public var outer_header: UInt8 = 0

    /// The 4 bits of `outer_header` from bit 0 (flag `ext_id`). A value
    /// wider than the flag is cut to it when written.
    public var extId: UInt8 {
        get { UInt8(truncatingIfNeeded: (outer_header >> 0) & 0xF) }
        set { outer_header = (outer_header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 4 of `outer_header` (flag `M`).
    public var M: Bool {
        get { (outer_header & 0x10) != 0 }
        set { outer_header = newValue ? (outer_header | 0x10) : (outer_header & ~0x10) }
    }

    /// The 2 bits of `outer_header` from bit 5 (flag `enc`). A value
    /// wider than the flag is cut to it when written.
    public var enc: UInt8 {
        get { UInt8(truncatingIfNeeded: (outer_header >> 5) & 0x3) }
        set { outer_header = (outer_header & ~(0x3 << 5)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 5) }
    }

    /// Bit 7 of `outer_header` (flag `Z`).
    public var Z: Bool {
        get { (outer_header & 0x80) != 0 }
        set { outer_header = newValue ? (outer_header | 0x80) : (outer_header & ~0x80) }
    }

    /// A `CodecZenohDeclExtKeyexprInner` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands, read from as many bytes as the document says an earlier field gives it, of which it need not take all.
    public var inner: CodecZenohDeclExtKeyexprInner = CodecZenohDeclExtKeyexprInner()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(outer_header: UInt8, inner: Inner_Source) {
        self.outer_header = outer_header
        self.inner = inner
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclExtKeyexpr {
        var reader = cursor
        let outer_header = try reader.readInteger(UInt8.self, .big)
        let total_length = try reader.readVle(UInt64.self)
        let inner_inner_length = Int(clamping: total_length)
        var inner_inner = SceCursor(Array(try reader.peekSlice(inner_inner_length)))
        let inner = try CodecZenohDeclExtKeyexprInner.decode(&inner_inner)
        try reader.advance(inner_inner_length)
        cursor = reader
        return CodecZenohDeclExtKeyexpr(outer_header: outer_header, inner: inner)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohDeclExtKeyexpr` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `outer_header`, where a parent sets it.
        public var outerHeaderM: Bool?
        /// The `Z` flag of `outer_header`, where a parent sets it.
        public var outerHeaderZ: Bool?

        public init(outerHeaderM: Bool? = nil, outerHeaderZ: Bool? = nil) {
            self.outerHeaderM = outerHeaderM
            self.outerHeaderZ = outerHeaderZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohDeclExtKeyexprSource` is written,
    /// with no copy into `CodecZenohDeclExtKeyexpr`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohDeclExtKeyexprSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var outer_header_folded = source.outer_header
        if let set = overrides.outerHeaderM { outer_header_folded = set ? (outer_header_folded | 0x10) : (outer_header_folded & ~0x10) }
        if let set = overrides.outerHeaderZ { outer_header_folded = set ? (outer_header_folded | 0x80) : (outer_header_folded & ~0x80) }
        var inner_piece = BytesSink()
        try CodecZenohDeclExtKeyexprInner.encodeSource(source.inner, into: &inner_piece)
        let inner_bytes = inner_piece.bytes
        let total_length_derived: UInt64 = UInt64(inner_bytes.count)
        var frame: [UInt8] = []
        appendInteger(outer_header_folded, to: &frame, .big)
        appendVle(total_length_derived, to: &frame)
        frame += inner_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
