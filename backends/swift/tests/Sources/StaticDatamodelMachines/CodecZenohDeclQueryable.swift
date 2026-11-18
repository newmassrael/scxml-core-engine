// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_decl_queryable.scxml:46 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecZenohDeclQueryable` the gates `Z` decide together (docs/adr/0065 §6
/// R4b, R4c): the elements present while the gates are as the one arm says. The encode derives the gates
/// from whether a value holds it, and a decode makes the arm the gates of the frame name.
public struct CodecZenohDeclQueryableZ: Equatable, Sendable {
    public var ext_type: UInt8
    public var ext_value: UInt64

    public init(ext_type: UInt8, ext_value: UInt64) {
        self.ext_type = ext_type
        self.ext_value = ext_value
    }
}

/// The elements a value of `CodecZenohDeclQueryable` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohDeclQueryable.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohDeclQueryable`, which conforms itself.
public protocol CodecZenohDeclQueryableSource {
    /// What `wireexpr` is presented as: a source of `CodecZenohWireexpr`, written by its encode.
    associatedtype Wireexpr_Source: CodecZenohWireexprSource
    var id: UInt32 { get }
    var wireexpr: Wireexpr_Source { get }
    var z: CodecZenohDeclQueryableZ? { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclQueryable: Equatable, Sendable, CodecZenohDeclQueryableSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohWireexpr.frameLength

    /// The value presents `wireexpr` as a value of its own codec.
    public typealias Wireexpr_Source = CodecZenohWireexpr

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt32 = 0

    /// A `CodecZenohWireexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var wireexpr: CodecZenohWireexpr = CodecZenohWireexpr()

    /// The elements the gates `Z` decide together, `nil` while they name no arm.
    public var z: CodecZenohDeclQueryableZ? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt32, wireexpr: Wireexpr_Source, z: CodecZenohDeclQueryableZ?) {
        self.id = id
        self.wireexpr = wireexpr
        self.z = z
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, N: UInt8, Z: UInt8) throws(CodecError) -> CodecZenohDeclQueryable {
        var reader = cursor
        let id = try reader.readVle(UInt32.self)
        let wireexpr = try CodecZenohWireexpr.decode(&reader, N: N)
        var ext_type: UInt8? = nil
        if (Z & 0x1) != 0 {
            ext_type = try reader.readInteger(UInt8.self, .big)
        }
        var ext_value: UInt64? = nil
        if (Z & 0x1) != 0 {
            ext_value = try reader.readVle(UInt64.self)
        }
        let z_group: CodecZenohDeclQueryableZ?
        if (Z & 0x01) != 0 {
            z_group = CodecZenohDeclQueryableZ(ext_type: ext_type!, ext_value: ext_value!)
        } else {
            z_group = nil
        }
        cursor = reader
        return CodecZenohDeclQueryable(id: id, wireexpr: wireexpr, z: z_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohDeclQueryableSource` is written,
    /// with no copy into `CodecZenohDeclQueryable`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohDeclQueryableSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var ext_type_member: UInt8? = nil
        var ext_value_member: UInt64? = nil
        if let held = source.z { ext_type_member = held.ext_type; ext_value_member = held.ext_value }
        var wireexpr_piece = BytesSink()
        try CodecZenohWireexpr.encodeSource(source.wireexpr, into: &wireexpr_piece)
        let wireexpr_bytes = wireexpr_piece.bytes
        var frame: [UInt8] = []
        appendVle(source.id, to: &frame)
        frame += wireexpr_bytes
        if let held = ext_type_member {
            appendInteger(held, to: &frame, .big)
        }
        if let held = ext_value_member {
            appendVle(held, to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `N` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputN<Source: CodecZenohDeclQueryableSource>(_ source: Source) -> UInt8 {
        CodecZenohWireexpr.inputN(source.wireexpr)
    }

    /// The flag input `Z` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputZ<Source: CodecZenohDeclQueryableSource>(_ source: Source) -> UInt8 {
        (source.z != nil ? true : false) ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
