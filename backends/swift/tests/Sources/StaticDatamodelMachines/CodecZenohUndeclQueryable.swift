// SCE-GENERATED — DO NOT EDIT
// source-hash: 54d1000cca396a272df15d0da82fa774aaa7c2162b420b3570b4082c8043d6e0
// source: codec_zenoh_undecl_queryable.scxml
// SCE-MAP: codec_zenoh_undecl_queryable.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohUndeclQueryable` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohUndeclQueryable.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohUndeclQueryable`, which conforms itself.
public protocol CodecZenohUndeclQueryableSource {
    /// What `ext_keyexpr` is presented as: a source of `CodecZenohDeclExtKeyexpr`, written by its encode.
    associatedtype ExtKeyexpr_Source: CodecZenohDeclExtKeyexprSource
    var id: UInt32 { get }
    var ext_keyexpr: ExtKeyexpr_Source? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohUndeclQueryable: Equatable, Sendable, CodecZenohUndeclQueryableSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The value presents `ext_keyexpr` as a value of its own codec.
    public typealias ExtKeyexpr_Source = CodecZenohDeclExtKeyexpr

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt32 = 0

    /// On the wire only when `(Z & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecZenohDeclExtKeyexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var ext_keyexpr: CodecZenohDeclExtKeyexpr? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt32, ext_keyexpr: ExtKeyexpr_Source?) {
        self.id = id
        self.ext_keyexpr = ext_keyexpr
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, Z: UInt8) throws(CodecError) -> CodecZenohUndeclQueryable {
        var reader = cursor
        let id = try reader.readVle(UInt32.self)
        var ext_keyexpr: CodecZenohDeclExtKeyexpr? = nil
        if (Z & 0x1) != 0 {
            ext_keyexpr = try CodecZenohDeclExtKeyexpr.decode(&reader)
        }
        cursor = reader
        return CodecZenohUndeclQueryable(id: id, ext_keyexpr: ext_keyexpr)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohUndeclQueryableSource` is written,
    /// with no copy into `CodecZenohUndeclQueryable`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohUndeclQueryableSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var ext_keyexpr_bytes: [UInt8]? = nil
        if let held = source.ext_keyexpr {
            var piece = BytesSink()
            try CodecZenohDeclExtKeyexpr.encodeSource(held, into: &piece)
            ext_keyexpr_bytes = piece.bytes
        }
        var frame: [UInt8] = []
        appendVle(source.id, to: &frame)
        if let held = ext_keyexpr_bytes { frame += held }
        try sink.writeBytes(frame)
    }

    /// The flag input `Z` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputZ<Source: CodecZenohUndeclQueryableSource>(_ source: Source) -> UInt8 {
        source.ext_keyexpr != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
