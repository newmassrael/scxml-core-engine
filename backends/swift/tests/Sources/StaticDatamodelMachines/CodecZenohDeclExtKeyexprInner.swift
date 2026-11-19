// SCE-GENERATED — DO NOT EDIT
// source-hash: b16d6bd3e09a98ce4bf2731362becaac44548a4aa0cc8e39c9e60e4b15d031d1
// source: codec_zenoh_decl_ext_keyexpr_inner.scxml
// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner.scxml:65 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohDeclExtKeyexprInner` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohDeclExtKeyexprInner.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohDeclExtKeyexprInner`, which conforms itself.
public protocol CodecZenohDeclExtKeyexprInnerSource {
    var inner_header: UInt8 { get }
    var id: UInt64 { get }
    var suffix: BoundedBytes<CodecZenohDeclExtKeyexprInner.SuffixBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclExtKeyexprInner: Equatable, Sendable, CodecZenohDeclExtKeyexprInnerSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `suffix` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum SuffixBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 65535
    }

    /// The field at byte 0, big-endian.
    public var inner_header: UInt8 = 0

    /// Bit 0 of `inner_header` (flag `N`).
    public var N: Bool {
        get { (inner_header & 0x1) != 0 }
        set { inner_header = newValue ? (inner_header | 0x1) : (inner_header & ~0x1) }
    }

    /// Bit 1 of `inner_header` (flag `M`).
    public var M: Bool {
        get { (inner_header & 0x2) != 0 }
        set { inner_header = newValue ? (inner_header | 0x2) : (inner_header & ~0x2) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt64 = 0

    /// On the wire only when `(inner_header & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, and a value with more is no value of its type.
    public var suffix: BoundedBytes<CodecZenohDeclExtKeyexprInner.SuffixBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(inner_header: UInt8, id: UInt64, suffix: BoundedBytes<CodecZenohDeclExtKeyexprInner.SuffixBound>?) {
        self.inner_header = inner_header
        self.id = id
        self.suffix = suffix
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclExtKeyexprInner {
        var reader = cursor
        let inner_header = try reader.readInteger(UInt8.self, .big)
        let id = try reader.readVle(UInt64.self)
        var suffix: BoundedBytes<CodecZenohDeclExtKeyexprInner.SuffixBound>? = nil
        if (inner_header & 0x1) != 0 {
            if reader.remaining > 65535 {
                throw CodecError.lengthOutOfRange
            }
            suffix = try BoundedBytes<CodecZenohDeclExtKeyexprInner.SuffixBound>(decoding: Array(try reader.readSlice(reader.remaining)))
        }
        cursor = reader
        return CodecZenohDeclExtKeyexprInner(inner_header: inner_header, id: id, suffix: suffix)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohDeclExtKeyexprInner` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `inner_header`, where a parent sets it.
        public var innerHeaderM: Bool?

        public init(innerHeaderM: Bool? = nil) {
            self.innerHeaderM = innerHeaderM
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohDeclExtKeyexprInnerSource` is written,
    /// with no copy into `CodecZenohDeclExtKeyexprInner`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohDeclExtKeyexprInnerSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var inner_header_folded = source.inner_header
        inner_header_folded = (inner_header_folded & ~0x1) | ((source.suffix != nil) ? 0x1 : 0)
        if let set = overrides.innerHeaderM { inner_header_folded = set ? (inner_header_folded | 0x2) : (inner_header_folded & ~0x2) }
        var frame: [UInt8] = []
        appendInteger(inner_header_folded, to: &frame, .big)
        appendVle(source.id, to: &frame)
        if let held = source.suffix {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
