// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes (SCE_FORGE.md §4.6.1): one
// definite-length map whose keys are small unsigned integers and whose values are unsigned integers,
// booleans, text strings and byte strings, or a definite-length array of maps, each a record keyed by
// its fields' ids as text (docs/adr/0086, section 9.2).
//
// Siblings, rule for rule: `codec.CborRead*` (Go, `backends/go/forge-runtime/codec/cbor.go`) and the
// `Cbor` object (Kotlin). A generated codec calls these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest form, so two backends given
// the same value write the same bytes. A write appends to the frame the generated code builds, which it
// hands to the sink in one write, so an encode that is refused has written nothing. Reading takes a head
// in any valid length, since what it reads is the value, and refuses what the codec cannot read as its
// entry: a reserved additional-information value, an indefinite length, another major type. A read moves
// the cursor it is given, and the generated decode gives it a copy that replaces the caller's only when
// the whole map was read.

/// The CBOR items of a codec map.
public enum Cbor {
    /// Major type 0, an unsigned integer.
    public static let majorUnsigned: UInt8 = 0
    /// Major type 2, a byte string.
    public static let majorBytes: UInt8 = 2
    /// Major type 3, a UTF-8 text string.
    public static let majorText: UInt8 = 3
    /// Major type 4, an array.
    public static let majorArray: UInt8 = 4
    /// Major type 5, a map.
    public static let majorMap: UInt8 = 5
    /// Major type 7, simple values (false is 20, true is 21).
    public static let majorSimple: UInt8 = 7
    /// The deepest an unknown entry's value may nest before a skip refuses it.
    public static let maxSkipDepth = 16

    // MARK: Writing

    /// Appends a head of `major` carrying `value`, in its shortest form.
    public static func writeHead(_ major: UInt8, _ value: UInt64, to buffer: inout [UInt8]) {
        let initial = major << 5
        switch value {
        case 0..<24:
            buffer.append(initial | UInt8(value))
        case 24...0xFF:
            buffer.append(initial | 24)
            buffer.append(UInt8(value))
        case 0x100...0xFFFF:
            buffer.append(initial | 25)
            appendInteger(UInt16(value), to: &buffer, .big)
        case 0x1_0000...0xFFFF_FFFF:
            buffer.append(initial | 26)
            appendInteger(UInt32(value), to: &buffer, .big)
        default:
            buffer.append(initial | 27)
            appendInteger(value, to: &buffer, .big)
        }
    }

    /// Appends an unsigned integer.
    public static func writeUint(_ value: UInt64, to buffer: inout [UInt8]) {
        writeHead(majorUnsigned, value, to: &buffer)
    }

    /// Appends false or true.
    public static func writeBool(_ value: Bool, to buffer: inout [UInt8]) {
        buffer.append(majorSimple << 5 | (value ? 21 : 20))
    }

    /// Appends a text string.
    public static func writeText(_ value: String, to buffer: inout [UInt8]) {
        let bytes = value.utf8
        writeHead(majorText, UInt64(bytes.count), to: &buffer)
        buffer.append(contentsOf: bytes)
    }

    /// Appends a byte string.
    public static func writeBytes(_ value: [UInt8], to buffer: inout [UInt8]) {
        writeHead(majorBytes, UInt64(value.count), to: &buffer)
        buffer.append(contentsOf: value)
    }

    /// Appends the head of a definite-length array of `items` items: a list of records an entry holds.
    public static func writeArrayHead(_ items: UInt64, to buffer: inout [UInt8]) {
        writeHead(majorArray, items, to: &buffer)
    }

    /// Appends the head of a definite-length map of `entries` entries.
    public static func writeMapHead(_ entries: UInt64, to buffer: inout [UInt8]) {
        writeHead(majorMap, entries, to: &buffer)
    }

    // MARK: Reading

    /// Reads one head: its major type and the value its additional information carries (for a string or
    /// a map, the length). An indefinite length and the reserved values 28 to 30 are refused.
    public static func readHead(
        _ cursor: inout SceCursor
    ) throws(CodecError) -> (major: UInt8, value: UInt64) {
        let initial = try cursor.readInteger(UInt8.self, .big)
        let major = initial >> 5
        let info = initial & 0x1F
        switch info {
        case 0..<24:
            return (major, UInt64(info))
        case 24:
            return (major, UInt64(try cursor.readInteger(UInt8.self, .big)))
        case 25:
            return (major, UInt64(try cursor.readInteger(UInt16.self, .big)))
        case 26:
            return (major, UInt64(try cursor.readInteger(UInt32.self, .big)))
        case 27:
            return (major, try cursor.readInteger(UInt64.self, .big))
        default:
            throw .cborMalformed
        }
    }

    private static func expect(_ cursor: inout SceCursor, _ major: UInt8) throws(CodecError) -> UInt64 {
        let head = try readHead(&cursor)
        guard head.major == major else {
            throw .cborMalformed
        }
        return head.value
    }

    /// Reads an unsigned integer.
    public static func readUint(_ cursor: inout SceCursor) throws(CodecError) -> UInt64 {
        try expect(&cursor, majorUnsigned)
    }

    /// Reads an unsigned integer an entry of `max` holds.
    public static func readUintUpto(_ cursor: inout SceCursor, _ max: UInt64) throws(CodecError) -> UInt64 {
        let value = try readUint(&cursor)
        guard value <= max else {
            throw .cborOutOfRange
        }
        return value
    }

    /// Reads false or true.
    public static func readBool(_ cursor: inout SceCursor) throws(CodecError) -> Bool {
        let head = try readHead(&cursor)
        guard head.major == majorSimple, head.value == 20 || head.value == 21 else {
            throw .cborMalformed
        }
        return head.value == 21
    }

    /// The `length` bytes of a string, which the cursor moves past: a length past the end of the input
    /// is a map that is not whole yet.
    private static func readPayload(
        _ cursor: inout SceCursor, _ length: UInt64
    ) throws(CodecError) -> ArraySlice<UInt8> {
        guard length <= UInt64(cursor.remaining) else {
            throw .needMoreBytes
        }
        return try cursor.readSlice(Int(length))
    }

    /// Reads a UTF-8 text string of at most `maxSize` bytes (`nil`: no bound). The bound is judged on
    /// the length the map says, before the bytes are looked for.
    public static func readText(_ cursor: inout SceCursor, maxSize: Int?) throws(CodecError) -> String {
        let length = try expect(&cursor, majorText)
        if let maxSize, length > UInt64(maxSize) {
            throw .cborOutOfRange
        }
        guard length <= UInt64(cursor.remaining) else {
            throw .needMoreBytes
        }
        return try cursor.readString(Int(length))
    }

    /// Reads a byte string of at most `maxSize` bytes (`nil`: no bound), judged on the length the map
    /// says before the bytes are looked for.
    public static func readBytes(_ cursor: inout SceCursor, maxSize: Int?) throws(CodecError) -> [UInt8] {
        let length = try expect(&cursor, majorBytes)
        if let maxSize, length > UInt64(maxSize) {
            throw .cborOutOfRange
        }
        return Array(try readPayload(&cursor, length))
    }

    /// Reads a byte string of exactly `length` bytes.
    public static func readBytesExact(_ cursor: inout SceCursor, length: UInt64) throws(CodecError) -> [UInt8] {
        let said = try expect(&cursor, majorBytes)
        guard said == length else {
            throw .cborWrongLength
        }
        return Array(try readPayload(&cursor, said))
    }

    /// Reads the head of a definite-length array: how many items follow. An indefinite length is
    /// refused, as a map's is.
    public static func readArrayLen(_ cursor: inout SceCursor) throws(CodecError) -> UInt64 {
        try expect(&cursor, majorArray)
    }

    /// Reads the head of a definite-length map: how many entries follow.
    public static func readMapLen(_ cursor: inout SceCursor) throws(CodecError) -> UInt64 {
        try expect(&cursor, majorMap)
    }

    /// Skips one item, the value of a key the codec does not declare, and everything nested in it,
    /// refusing one nested deeper than `maxSkipDepth`.
    public static func skip(_ cursor: inout SceCursor) throws(CodecError) {
        try skip(&cursor, depth: 0)
    }

    private static func skip(_ cursor: inout SceCursor, depth: Int) throws(CodecError) {
        guard depth < maxSkipDepth else {
            throw .cborTooDeep
        }
        let head = try readHead(&cursor)
        switch head.major {
        case 0, 1, 7:
            // Unsigned, negative, simple or float: the head is the whole item.
            break
        case 2, 3:
            _ = try readPayload(&cursor, head.value)
        case 4:
            var index: UInt64 = 0
            while index < head.value {
                try skip(&cursor, depth: depth + 1)
                index += 1
            }
        case 5:
            var index: UInt64 = 0
            while index < head.value {
                try skip(&cursor, depth: depth + 1)
                try skip(&cursor, depth: depth + 1)
                index += 1
            }
        case 6:
            // A tag: its one enclosed item follows.
            try skip(&cursor, depth: depth + 1)
        default:
            throw .cborMalformed
        }
    }
}
