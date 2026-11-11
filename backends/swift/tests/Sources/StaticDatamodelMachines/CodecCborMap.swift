// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_cbor_map.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and skips one this codec
/// does not declare; encode writes the entries present in ascending key order, every head in its
/// shortest form. A required entry starts at its type's zero and an optional one absent (`nil`), so
/// `CodecCborMap()` is the infallible initializer every codec offers.
public struct CodecCborMap: Equatable, Sendable {
    /// Map key 1, required.
    public var source: String = ""

    /// Map key 0, required.
    public var id: [UInt8] = []

    /// Map key 3, required.
    public var pattern: UInt8 = 0

    /// Map key 5, required.
    public var payload: [UInt8] = []

    /// Map key 12.
    public var deadline: UInt64? = nil

    /// Map key 7.
    public var urgent: Bool? = nil

    public init() {}

    public init(source: String, id: [UInt8], pattern: UInt8, payload: [UInt8], deadline: UInt64?, urgent: Bool?) {
        self.source = source
        self.id = id
        self.pattern = pattern
        self.payload = payload
        self.deadline = deadline
        self.urgent = urgent
    }

    /// Decodes one map at the cursor and moves the cursor past it. The map is read through a copy of
    /// the cursor, which replaces it only when the whole map was read: a map that is short answers
    /// `needMoreBytes`, one that is not a map this codec reads `cborMalformed`, and either leaves the
    /// cursor where it was.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecCborMap {
        var reader = cursor
        let count = try Cbor.readMapLen(&reader)
        var source_held: String? = nil
        var id_held: [UInt8]? = nil
        var pattern_held: UInt8? = nil
        var payload_held: [UInt8]? = nil
        var deadline_held: UInt64? = nil
        var urgent_held: Bool? = nil
        var index: UInt64 = 0
        while index < count {
            index += 1
            switch try Cbor.readUint(&reader) {
            case 0:
                // A key given twice is not one map entry.
                if id_held != nil { throw CodecError.cborMalformed }
                id_held = try Cbor.readBytesExact(&reader, length: 16)
            case 1:
                // A key given twice is not one map entry.
                if source_held != nil { throw CodecError.cborMalformed }
                source_held = try Cbor.readText(&reader, maxSize: 64)
            case 3:
                // A key given twice is not one map entry.
                if pattern_held != nil { throw CodecError.cborMalformed }
                pattern_held = UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255))
            case 5:
                // A key given twice is not one map entry.
                if payload_held != nil { throw CodecError.cborMalformed }
                payload_held = try Cbor.readBytes(&reader, maxSize: 256)
            case 7:
                // A key given twice is not one map entry.
                if urgent_held != nil { throw CodecError.cborMalformed }
                urgent_held = try Cbor.readBool(&reader)
            case 12:
                // A key given twice is not one map entry.
                if deadline_held != nil { throw CodecError.cborMalformed }
                deadline_held = UInt64(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 18446744073709551615))
            default:
                try Cbor.skip(&reader)
            }
        }
        var frame = CodecCborMap()
        guard let source_value = source_held else { throw CodecError.cborRequiredKeyMissing }
        frame.source = source_value
        guard let id_value = id_held else { throw CodecError.cborRequiredKeyMissing }
        frame.id = id_value
        guard let pattern_value = pattern_held else { throw CodecError.cborRequiredKeyMissing }
        frame.pattern = pattern_value
        guard let payload_value = payload_held else { throw CodecError.cborRequiredKeyMissing }
        frame.payload = payload_value
        frame.deadline = deadline_held
        frame.urgent = urgent_held
        cursor = reader
        return frame
    }

    /// Writes the map to `sink` in one write: a value that breaks the bound its entry declares (an
    /// exact length, a maximum size) is refused and nothing is written, and a bounded sink that cannot
    /// hold all of it answers `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var count: UInt64 = 4
        if self.deadline != nil { count += 1 }
        if self.urgent != nil { count += 1 }
        Cbor.writeMapHead(count, to: &frame)
        do {
            let value = self.id
            Cbor.writeUint(0, to: &frame)
            if value.count != 16 { throw CodecError.cborWrongLength }
            Cbor.writeBytes(value, to: &frame)
        }
        do {
            let value = self.source
            Cbor.writeUint(1, to: &frame)
            if value.utf8.count > 64 { throw CodecError.cborOutOfRange }
            Cbor.writeText(value, to: &frame)
        }
        do {
            let value = self.pattern
            Cbor.writeUint(3, to: &frame)
            Cbor.writeUint(UInt64(value), to: &frame)
        }
        do {
            let value = self.payload
            Cbor.writeUint(5, to: &frame)
            if value.count > 256 { throw CodecError.cborOutOfRange }
            Cbor.writeBytes(value, to: &frame)
        }
        if let value = self.urgent {
            Cbor.writeUint(7, to: &frame)
            Cbor.writeBool(value, to: &frame)
        }
        if let value = self.deadline {
            Cbor.writeUint(12, to: &frame)
            Cbor.writeUint(UInt64(value), to: &frame)
        }
        try sink.writeBytes(frame)
    }
}
