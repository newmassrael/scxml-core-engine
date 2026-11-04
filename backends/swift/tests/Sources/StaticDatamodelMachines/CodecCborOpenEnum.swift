// SCE-GENERATED — DO NOT EDIT
// source-hash: b149b183ac9d4db0350d56ff44276f1c17fe34c40f7a81a42a837e0b75fbf912
// SCE-MAP: codec_cbor_open_enum.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and skips one this codec
/// does not declare; encode writes the entries present in ascending key order, every head in its
/// shortest form. A required entry starts at its type's zero and an optional one absent (`nil`), so
/// `CodecCborOpenEnum()` is the infallible initializer every codec offers.
public struct CodecCborOpenEnum: Equatable, Sendable {
    /// Map key 1, required.
    public var status: EnumUdsNrc = EnumUdsNrc.generalReject

    /// Map key 2.
    public var detail: EnumUdsNrc? = nil

    public init() {}

    public init(status: EnumUdsNrc, detail: EnumUdsNrc?) {
        self.status = status
        self.detail = detail
    }

    /// Decodes one map at the cursor and moves the cursor past it. The map is read through a copy of
    /// the cursor, which replaces it only when the whole map was read: a map that is short answers
    /// `needMoreBytes`, one that is not a map this codec reads `cborMalformed`, and either leaves the
    /// cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecCborOpenEnum {
        var reader = cursor
        let count = try Cbor.readMapLen(&reader)
        var status_held: EnumUdsNrc? = nil
        var detail_held: EnumUdsNrc? = nil
        var index: UInt64 = 0
        while index < count {
            index += 1
            switch try Cbor.readUint(&reader) {
            case 1:
                // A key given twice is not one map entry.
                if status_held != nil { throw CodecError.cborMalformed }
                // An open enum holds any number its carrier does, so every number is a value.
                status_held = EnumUdsNrc.fromUnderlying(UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255)))
            case 2:
                // A key given twice is not one map entry.
                if detail_held != nil { throw CodecError.cborMalformed }
                // An open enum holds any number its carrier does, so every number is a value.
                detail_held = EnumUdsNrc.fromUnderlying(UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255)))
            default:
                try Cbor.skip(&reader)
            }
        }
        var frame = CodecCborOpenEnum()
        guard let status_value = status_held else { throw CodecError.cborRequiredKeyMissing }
        frame.status = status_value
        frame.detail = detail_held
        cursor = reader
        return frame
    }

    /// Writes the map to `sink` in one write: a value that breaks the bound its entry declares (an
    /// exact length, a maximum size) is refused and nothing is written, and a bounded sink that cannot
    /// hold all of it answers `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var count: UInt64 = 1
        if self.detail != nil { count += 1 }
        Cbor.writeMapHead(count, to: &frame)
        do {
            let value = self.status
            Cbor.writeUint(1, to: &frame)
            Cbor.writeUint(UInt64(EnumUdsNrc.toUnderlying(value)), to: &frame)
        }
        if let value = self.detail {
            Cbor.writeUint(2, to: &frame)
            Cbor.writeUint(UInt64(EnumUdsNrc.toUnderlying(value)), to: &frame)
        }
        try sink.writeBytes(frame)
    }
}
