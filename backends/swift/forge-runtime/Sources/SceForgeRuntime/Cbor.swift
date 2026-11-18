// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads and writes (SCE_FORGE.md §4.6.1,
// docs/adr/0101): one definite-length map whose keys are small unsigned integers and whose values are
// values of the language, each in its one form: an integer, a float, a bool, a text or byte string,
// an array, a map keyed by text, and a union's map of one entry.
//
// Siblings, rule for rule: `codec.CborRead*` (Go, `backends/go/forge-runtime/codec/cbor.go`) and
// `sce_forge_runtime::cbor` (Rust, `backends/rust/forge-runtime/src/cbor.rs`). A generated codec calls
// these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest form and every float in the
// shortest precision that holds it exactly, so two backends given the same value write the same bytes.
// A write appends to the frame the generated code builds, which it hands to the sink in one write, so
// an encode that is refused has written nothing. Reading takes the profile of definite lengths
// (docs/adr/0101 §3.1), and within it every well-formed encoding of a value: a head in any valid
// length, a float in any precision that holds the value. It refuses each input by the class RFC 8949
// §1.2 puts it in (docs/adr/0101 §3.3): an item that is not well-formed or has an indefinite length is
// `cborMalformed`, an item of a major type, simple value or tag the place does not take is
// `cborWrongType`, and a value outside its type is `cborOutOfRange`. A read moves the cursor it is
// given, and the generated decode gives it a copy that replaces the caller's only when the whole map
// was read.

/// The CBOR items of a codec map.
public enum Cbor {
    /// Major type 0, an unsigned integer.
    public static let majorUnsigned: UInt8 = 0
    /// Major type 1, the negative integer `-1 - n` for the argument `n`.
    public static let majorNegative: UInt8 = 1
    /// Major type 2, a byte string.
    public static let majorBytes: UInt8 = 2
    /// Major type 3, a UTF-8 text string.
    public static let majorText: UInt8 = 3
    /// Major type 4, an array.
    public static let majorArray: UInt8 = 4
    /// Major type 5, a map.
    public static let majorMap: UInt8 = 5
    /// Major type 7, simple values (false is 20, true is 21) and floats.
    public static let majorSimple: UInt8 = 7
    /// The deepest an unknown entry's value may nest before a skip refuses it.
    public static let maxSkipDepth = 16

    /// The additional information of a half, a single and a double precision float under major type
    /// 7 (RFC 8949 §3.3).
    private static let infoHalf: UInt8 = 25
    private static let infoSingle: UInt8 = 26
    private static let infoDouble: UInt8 = 27

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

    /// Appends a signed integer: major type 0 for zero or more, major type 1 with the argument
    /// `-1 - n` below zero, each with its shortest head (RFC 8949 §3.1 and §4.2.1).
    public static func writeInt(_ value: Int64, to buffer: inout [UInt8]) {
        if value < 0 {
            // `-1 - value` is at most `Int64.max`: it neither overflows nor loses a bit as a UInt64.
            writeHead(majorNegative, UInt64(-1 - value), to: &buffer)
        } else {
            writeHead(majorUnsigned, UInt64(value), to: &buffer)
        }
    }

    /// The half precision float that is `x` exactly, as its bits, or `nil` when no half is (IEEE 754
    /// binary16: 5 bits of exponent, 10 of fraction, subnormal below 2^-14). A NaN has none.
    static func halfOf(_ x: Double) -> UInt16? {
        let bits = x.bitPattern
        let sign = UInt16(truncatingIfNeeded: (bits >> 48) & 0x8000)
        let exponent = Int((bits >> 52) & 0x7FF)
        let fraction: UInt64 = bits & ((1 << 52) - 1)
        if exponent == 0x7FF {
            // An infinity; a NaN is no half's.
            return fraction == 0 ? sign | 0x7C00 : nil
        }
        if exponent == 0 {
            // Zero, or a double's subnormal, which is far below a half's least.
            return fraction == 0 ? sign : nil
        }
        let power = exponent - 1023
        if (-14...15).contains(power) {
            // A half's normal number keeps the top 10 bits of the fraction and nothing below them.
            guard fraction & ((1 << 42) - 1) == 0 else {
                return nil
            }
            let biased = UInt16(power + 15) << 10
            return sign | biased | UInt16(truncatingIfNeeded: fraction >> 42)
        }
        if (-24 ... -15).contains(power) {
            // A half's subnormal number is k * 2^-24 for k from 1 to 1023.
            let significand: UInt64 = fraction | (1 << 52)
            let shift = UInt64(28 - power)
            guard significand & ((1 << shift) - 1) == 0 else {
                return nil
            }
            return sign | UInt16(truncatingIfNeeded: significand >> shift)
        }
        return nil
    }

    /// The single precision float that is `x` exactly, or `nil` when no single is. A NaN has none.
    static func singleOf(_ x: Double) -> Float? {
        let single = Float(x)
        return Double(single) == x ? single : nil
    }

    /// Appends a float in the shortest of half, single and double precision that holds it exactly,
    /// and a NaN as `f9 7e 00` (RFC 8949 §4.1 and §4.2.1, docs/adr/0086 section 9.2). A `float32` is
    /// written by its value as a Double, which a single always holds.
    public static func writeFloat(_ x: Double, to buffer: inout [UInt8]) {
        if x.isNaN {
            buffer.append(majorSimple << 5 | infoHalf)
            buffer.append(0x7E)
            buffer.append(0x00)
        } else if let half = halfOf(x) {
            buffer.append(majorSimple << 5 | infoHalf)
            appendInteger(half, to: &buffer, .big)
        } else if let single = singleOf(x) {
            buffer.append(majorSimple << 5 | infoSingle)
            appendInteger(single.bitPattern, to: &buffer, .big)
        } else {
            buffer.append(majorSimple << 5 | infoDouble)
            appendInteger(x.bitPattern, to: &buffer, .big)
        }
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

    /// Appends the head of a union's value: a map of one entry keyed by the case's variant name as a
    /// text string (docs/adr/0101 §2.1). The case's record, or the empty map for a case that holds
    /// none, is appended after it.
    public static func writeVariant(_ name: String, to buffer: inout [UInt8]) {
        writeMapHead(1, to: &buffer)
        writeText(name, to: &buffer)
    }

    // MARK: Reading

    /// Reads one head of the profile as the input wrote it: its major type, its additional
    /// information, which tells a float's precision apart, and its argument. What is not well-formed
    /// (RFC 8949 Appendix F) is refused: the reserved additional information 28 to 30, a simple value
    /// in the two-byte form below 32 (RFC 8949 §3.3), and a break code where an item stands; so is an
    /// indefinite length, which the profile does not hold (docs/adr/0101 §3.1). Each is
    /// `cborMalformed`.
    private static func readItemHead(
        _ cursor: inout SceCursor
    ) throws(CodecError) -> (major: UInt8, info: UInt8, argument: UInt64) {
        let initial = try cursor.readInteger(UInt8.self, .big)
        let major = initial >> 5
        let info = initial & 0x1F
        let argument: UInt64
        switch info {
        case 0..<24:
            argument = UInt64(info)
        case 24:
            argument = UInt64(try cursor.readInteger(UInt8.self, .big))
        case 25:
            argument = UInt64(try cursor.readInteger(UInt16.self, .big))
        case 26:
            argument = UInt64(try cursor.readInteger(UInt32.self, .big))
        case 27:
            argument = try cursor.readInteger(UInt64.self, .big)
        default:
            throw .cborMalformed
        }
        if major == majorSimple && info == 24 && argument < 32 {
            throw .cborMalformed
        }
        return (major, info, argument)
    }

    /// Reads one head: its major type and the value its additional information carries (for a string
    /// or a map, the length). An indefinite length, the reserved values 28 to 30 and a two-byte
    /// simple value below 32 are refused.
    public static func readHead(
        _ cursor: inout SceCursor
    ) throws(CodecError) -> (major: UInt8, value: UInt64) {
        let head = try readItemHead(&cursor)
        return (head.major, head.argument)
    }

    /// The argument of a head of `major`; a head of another major type is an item the place does
    /// not take.
    private static func expect(_ cursor: inout SceCursor, _ major: UInt8) throws(CodecError) -> UInt64 {
        let head = try readItemHead(&cursor)
        guard head.major == major else {
            throw .cborWrongType
        }
        return head.argument
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

    /// Reads a signed integer of `least...greatest`, the range of its type: major type 0, or major
    /// type 1 below zero. A number outside the range is `cborOutOfRange`, another item
    /// `cborWrongType`.
    public static func readIntWithin(
        _ cursor: inout SceCursor, _ least: Int64, _ greatest: Int64
    ) throws(CodecError) -> Int64 {
        let head = try readItemHead(&cursor)
        guard head.major == majorUnsigned || head.major == majorNegative else {
            throw .cborWrongType
        }
        // An argument past `Int64.max` is a number past every Int64.
        guard head.argument <= UInt64(Int64.max) else {
            throw .cborOutOfRange
        }
        let magnitude = Int64(head.argument)
        let value = head.major == majorNegative ? -1 - magnitude : magnitude
        guard value >= least && value <= greatest else {
            throw .cborOutOfRange
        }
        return value
    }

    /// The value of a half precision float's bits (IEEE 754 binary16), exactly.
    private static func fromHalf(_ bits: UInt16) -> Double {
        let exponent = Int((bits >> 10) & 0x1F)
        let fraction = Double(bits & 0x3FF)
        let magnitude: Double
        switch exponent {
        case 0:
            magnitude = fraction * 0x1p-24
        case 0x1F:
            magnitude = fraction == 0 ? Double.infinity : Double.nan
        default:
            magnitude = (1024 + fraction) * Double(sign: .plus, exponent: exponent - 25, significand: 1)
        }
        return bits & 0x8000 != 0 ? -magnitude : magnitude
    }

    /// Reads a float in any of half, single and double precision, as its value: a variation-tolerant
    /// read (RFC 8949 §4.1). An integer, a simple value and every other item are `cborWrongType`.
    public static func readFloat(_ cursor: inout SceCursor) throws(CodecError) -> Double {
        let head = try readItemHead(&cursor)
        guard head.major == majorSimple else {
            throw .cborWrongType
        }
        switch head.info {
        case infoHalf:
            return fromHalf(UInt16(truncatingIfNeeded: head.argument))
        case infoSingle:
            return Double(Float(bitPattern: UInt32(truncatingIfNeeded: head.argument)))
        case infoDouble:
            return Double(bitPattern: head.argument)
        default:
            throw .cborWrongType
        }
    }

    /// Reads a `float32`: a float in any precision whose value a single holds. A double that no single
    /// is exactly is `cborOutOfRange`; a NaN of any precision is a NaN.
    public static func readFloat32(_ cursor: inout SceCursor) throws(CodecError) -> Float {
        let x = try readFloat(&cursor)
        if x.isNaN {
            return Float.nan
        }
        guard let single = singleOf(x) else {
            throw .cborOutOfRange
        }
        return single
    }

    /// Reads false or true. Every other item, `null` and a float among them, is `cborWrongType`.
    public static func readBool(_ cursor: inout SceCursor) throws(CodecError) -> Bool {
        let head = try readItemHead(&cursor)
        guard head.major == majorSimple, head.info == 20 || head.info == 21 else {
            throw .cborWrongType
        }
        return head.info == 21
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

    /// Reads the head of a union's value and names its case (docs/adr/0101 §2.1 and §3.3): a map,
    /// judged for exactly one entry before its key is read (`cborUnionEntries`), whose key is a text
    /// string (`cborWrongType`) naming one of `names` (`cborUnknownVariant`). Answers the case's index
    /// in `names`; the case's record, or the empty map, follows.
    public static func readVariant(_ cursor: inout SceCursor, names: [String]) throws(CodecError) -> Int {
        guard try readMapLen(&cursor) == 1 else {
            throw .cborUnionEntries
        }
        let name = try readText(&cursor, maxSize: nil)
        guard let index = names.firstIndex(of: name) else {
            throw .cborUnknownVariant
        }
        return index
    }

    /// Skips one item, the value of a key the codec does not declare, and everything nested in it,
    /// refusing one nested deeper than `maxSkipDepth` and one the profile does not hold (an indefinite
    /// length at any depth).
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
