// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The content lines (RFC 5545 section 3.1) a `sce:encoding="content-line"` codec reads and writes
// (SCE_FORGE.md section 4.6.4, docs/adr/0010): `BEGIN:<component>`, properties (a name,
// `;`-separated parameters, `:` and a value) and `END:<component>`, folded at 75 octets.
//
// A generated codec calls these and spells no line grammar of its own. The rules are written once in
// SCE_FORGE.md section 4.6.4 and implemented once per backend runtime; this is Swift's, the sibling of
// the `ContentLine` object (Kotlin), `codec.ContentLine*` (Go) and `sce_forge_runtime::content_line`
// (Rust). Reading and writing throw the `CodecError` of the rule that refused. A write appends to the
// frame the writer holds, which the generated code hands to the sink in one write, so an encode that is
// refused has written nothing.

/// The content-line grammar a codec reads and writes.
public enum ContentLine {
    /// The most octets of one physical line (SCE_FORGE.md section 4.6.4, *Folding*).
    public static let foldWidth = 75

    /// The bound a reader or a writer is handed for a text or a list the protocol states no bound of
    /// (docs/adr/0077 section 7): the largest `Int`, past which no frame this runtime reads has a
    /// length.
    public static let unbounded = Int.max

    /// A control character a value never holds: below U+0020 but the tab, and U+007F.
    static func isControl(_ b: Int) -> Bool { (b < 0x20 && b != 0x09) || b == 0x7F }

    /// A byte of a property or parameter name: letters, digits and hyphens.
    static func isNameByte(_ b: Int) -> Bool {
        (b >= 0x30 && b <= 0x39) || (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) || b == 0x2D
    }

    /// The byte with an ASCII capital folded to its small letter, and every other byte as it is.
    static func lower(_ b: Int) -> Int { (b >= 0x41 && b <= 0x5A) ? b + 0x20 : b }

    /// Whether the unfolded text of `raw[from..<to]` is `expected`, without regard to case.
    static func unfoldedEq(_ raw: [UInt8], _ from: Int, _ to: Int, _ expected: String) -> Bool {
        var scan = Scan(raw: raw, limit: to, pos: from)
        for ch in expected.utf8 {
            let b = scan.bump()
            if b < 0 || lower(b) != lower(Int(ch)) { return false }
        }
        return scan.peek() < 0
    }

    /// The text of `bytes`, which a value that is not well-formed UTF-8 has none of.
    static func utf8(_ bytes: [UInt8]) throws(CodecError) -> String {
        guard let text = String(validating: bytes, as: UTF8.self) else { throw .lineBadValue }
        return text
    }
}

private let cr: UInt8 = 13
private let lf: UInt8 = 10
private let space: UInt8 = 32
private let tab: UInt8 = 9

private func ascii(_ c: Unicode.Scalar) -> Int { Int(c.value) }

/// A walk over the bytes of one logical line, `raw[pos..<limit]`, with its folds removed as it goes:
/// a line break and the one space or tab after it are not part of the text, wherever the sender cut.
struct Scan {
    let raw: [UInt8]
    let limit: Int
    var pos: Int

    private func blank(_ i: Int) -> Bool { i < limit && (raw[i] == space || raw[i] == tab) }

    /// Steps over every fold at the current position.
    mutating func skipFolds() {
        while true {
            let skipped: Int
            if pos + 1 < limit && raw[pos] == cr && raw[pos + 1] == lf && blank(pos + 2) {
                skipped = 3
            } else if pos < limit && raw[pos] == lf && blank(pos + 1) {
                skipped = 2
            } else {
                skipped = 0
            }
            if skipped == 0 { return }
            pos += skipped
        }
    }

    /// The next byte as 0...255 without taking it, or -1 at the end of the line.
    mutating func peek() -> Int {
        skipFolds()
        return pos < limit ? Int(raw[pos]) : -1
    }

    mutating func bump() -> Int {
        let b = peek()
        if b >= 0 { pos += 1 }
        return b
    }
}

/// One logical line of the input, with its folds still inside it.
private struct RawLine {
    let from: Int
    let to: Int
    let terminated: Bool
}

/// The name of a line and what follows it, as indexes into the input.
private struct Head {
    let nameEnd: Int
    let separator: Int
    let restStart: Int
}

private func headOf(_ raw: [UInt8], _ from: Int, _ to: Int) -> Head? {
    var scan = Scan(raw: raw, limit: to, pos: from)
    var named = false
    while true {
        let b = scan.peek()
        if b >= 0 && ContentLine.isNameByte(b) {
            _ = scan.bump()
            named = true
        } else {
            break
        }
    }
    if !named { return nil }
    let nameEnd = scan.pos
    let separator = scan.peek()
    if separator != ascii(";") && separator != ascii(":") { return nil }
    _ = scan.bump()
    return Head(nameEnd: nameEnd, separator: separator, restStart: scan.pos)
}

// MARK: Reading

/// A reader over the properties of one component.
public struct ContentLineReader {
    private let input: [UInt8]
    private let component: String
    private var pos = 0
    /// How many nested components the walk is inside (`VALARM` in `VEVENT`).
    private var depth = 0

    private init(input: [UInt8], component: String) {
        self.input = input
        self.component = component
    }

    /// How many bytes of the input the walk has passed: after `END:<component>` once `next()` has
    /// answered `nil`.
    public var consumed: Int { pos }

    private mutating func nextRawLine() -> RawLine? {
        if pos >= input.count { return nil }
        let start = pos
        var i = start
        while i < input.count {
            let folded = i + 1 < input.count && (input[i + 1] == space || input[i + 1] == tab)
            if input[i] == lf && !folded {
                let end = (i > start && input[i - 1] == cr) ? i - 1 : i
                pos = i + 1
                return RawLine(from: start, to: end, terminated: true)
            }
            i += 1
        }
        pos = input.count
        return RawLine(from: start, to: input.count, terminated: false)
    }

    /// The next property line of the component, or `nil` after its `END:<component>`.
    ///
    /// A nested component is skipped through its `END:` without reading its lines. An input that
    /// ends before `END:<component>` is `needMoreBytes`.
    public mutating func next() throws(CodecError) -> ContentLineProperty? {
        while true {
            guard let line = nextRawLine() else { throw .needMoreBytes }
            let head = headOf(input, line.from, line.to)
            if depth > 0 {
                if let head, head.separator == ascii(":") {
                    if ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN") {
                        depth += 1
                    } else if ContentLine.unfoldedEq(input, line.from, head.nameEnd, "END") {
                        depth -= 1
                    }
                }
                continue
            }
            // A line with no name is cut short at the end of the input and malformed anywhere else.
            let cut: CodecError = line.terminated ? .lineMalformed : .needMoreBytes
            guard let head else { throw cut }
            if head.separator == ascii(":") {
                if ContentLine.unfoldedEq(input, line.from, head.nameEnd, "END") {
                    if ContentLine.unfoldedEq(input, head.restStart, line.to, component) { return nil }
                    throw cut
                }
                if line.terminated && ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN") {
                    depth = 1
                    continue
                }
            }
            if !line.terminated { throw .needMoreBytes }
            return ContentLineProperty(raw: input, from: line.from, to: line.to, nameEnd: head.nameEnd)
        }
    }

    /// Skips lines up to the first `BEGIN:<component>` and stands after it. The input that ends
    /// before it is `needMoreBytes`.
    public static func begin(_ input: [UInt8], component: String) throws(CodecError) -> ContentLineReader {
        var reader = ContentLineReader(input: input, component: component)
        while let line = reader.nextRawLine() {
            if !line.terminated { break }
            guard let head = headOf(input, line.from, line.to) else { continue }
            if head.separator == ascii(":")
                && ContentLine.unfoldedEq(input, line.from, head.nameEnd, "BEGIN")
                && ContentLine.unfoldedEq(input, head.restStart, line.to, component)
            {
                return reader
            }
        }
        throw .needMoreBytes
    }
}

/// One property line the codec has been handed.
///
/// Ask `isNamed` whether it is one the codec reads. Then, for a property that declares parameters, loop
/// on `nextParam()` and read the ones the codec declares; then read the value. A parameter no one reads
/// is skipped by the next call, and a property read by value alone skips them all. Every read throws
/// for what the entry cannot hold.
public struct ContentLineProperty {
    private let raw: [UInt8]
    private let from: Int
    private let nameEnd: Int
    private var scan: Scan
    private var paramFrom = -1
    private var paramTo = -1
    private var phase = Phase.atSeparator

    /// Where the property stands in its line.
    private enum Phase {
        /// At the `;` that opens a parameter or the `:` that opens the value.
        case atSeparator
        /// After a parameter's `=`, before its value.
        case paramValue
        /// The value has been read.
        case done
    }

    init(raw: [UInt8], from: Int, to: Int, nameEnd: Int) {
        self.raw = raw
        self.from = from
        self.nameEnd = nameEnd
        self.scan = Scan(raw: raw, limit: to, pos: nameEnd)
    }

    /// Whether this property is `name`, without regard to case.
    public func isNamed(_ name: String) -> Bool {
        ContentLine.unfoldedEq(raw, from, nameEnd, name)
    }

    /// Whether the parameter `nextParam()` stands on is `name`, without regard to case.
    public func paramNamed(_ name: String) -> Bool {
        paramFrom >= 0 && ContentLine.unfoldedEq(raw, paramFrom, paramTo, name)
    }

    /// Stands on the next parameter, skipping the value of one not read: `true` when there is one,
    /// `false` when the value is next, and a throw when the line does not admit it.
    public mutating func nextParam() throws(CodecError) -> Bool {
        switch phase {
        case .paramValue: try skipParamValue()
        case .done: throw .lineMalformed
        case .atSeparator: break
        }
        switch scan.peek() {
        case ascii(":"):
            return false
        case ascii(";"):
            _ = scan.bump()
            _ = scan.peek()
            let start = scan.pos
            while true {
                let b = scan.peek()
                if b >= 0 && ContentLine.isNameByte(b) { _ = scan.bump() } else { break }
            }
            let end = scan.pos
            if end == start || scan.bump() != ascii("=") { throw .lineMalformed }
            paramFrom = start
            paramTo = end
            phase = .paramValue
            return true
        default:
            throw .lineMalformed
        }
    }

    /// Scans one parameter value (a quoted string, or text up to `;`, `:`, `,` or `"`), handing each
    /// byte of it to `emit`. Answers `true` when another value follows a `,` and `false` when none
    /// does.
    private mutating func scanParamValue(_ emit: (Int) throws(CodecError) -> Void) throws(CodecError) -> Bool {
        if scan.peek() == ascii("\"") {
            _ = scan.bump()
            while true {
                let b = scan.bump()
                if b < 0 { throw .lineMalformed }
                if b == ascii("\"") { break }
                try emit(b)
            }
        } else {
            while true {
                let b = scan.peek()
                if b < 0 || b == ascii(";") || b == ascii(":") || b == ascii(",") { break }
                if b == ascii("\"") { throw .lineMalformed }
                _ = scan.bump()
                try emit(b)
            }
        }
        switch scan.peek() {
        case ascii(","):
            _ = scan.bump()
            return true
        case ascii(";"), ascii(":"):
            return false
        default:
            throw .lineMalformed
        }
    }

    private mutating func skipParamValue() throws(CodecError) {
        while try scanParamValue({ _ in }) {}
        paramFrom = -1
        phase = .atSeparator
    }

    /// Reads the value of the parameter `nextParam()` stands on, into at most `maxSize` bytes. A
    /// second value is `lineBadValue`.
    public mutating func readParamString(maxSize: Int) throws(CodecError) -> String {
        if phase != .paramValue { throw .lineMalformed }
        var buf: [UInt8] = []
        let more = try scanParamValue { b throws(CodecError) in
            if ContentLine.isControl(b) { throw .lineBadValue }
            if buf.count == maxSize { throw .lineTooLong }
            buf.append(UInt8(b))
        }
        if more { throw .lineBadValue }
        paramFrom = -1
        phase = .atSeparator
        return try ContentLine.utf8(buf)
    }

    /// Stands at the value: skips the parameters still unread and steps over `:`.
    private mutating func beginValue() throws(CodecError) {
        while try nextParam() {}
        _ = scan.bump()
        phase = .done
    }

    /// The byte a backslash escape stands for in a TEXT: `\\`, `\;`, `\,`, `\n` and `\N`.
    private mutating func unescaped() throws(CodecError) -> UInt8 {
        switch scan.bump() {
        case ascii("\\"): return UInt8(ascii("\\"))
        case ascii(";"): return UInt8(ascii(";"))
        case ascii(","): return UInt8(ascii(","))
        case ascii("n"), ascii("N"): return 0x0A
        default: throw .lineBadEscape
        }
    }

    /// Reads the value as a `string` of at most `maxSize` bytes. With `text`, `\\`, `\;`, `\,`, `\n`
    /// and `\N` are escapes; an unescaped `;` or `,` is itself.
    public mutating func readString(maxSize: Int, text: Bool) throws(CodecError) -> String {
        try beginValue()
        var buf: [UInt8] = []
        while true {
            let b = scan.bump()
            if b < 0 { break }
            let byte: UInt8
            if text && b == ascii("\\") {
                byte = try unescaped()
            } else if ContentLine.isControl(b) {
                throw .lineBadValue
            } else {
                byte = UInt8(b)
            }
            if buf.count == maxSize { throw .lineTooLong }
            buf.append(byte)
        }
        return try ContentLine.utf8(buf)
    }

    /// Reads the value as a list of `string` cut at `separator`, at most `maxValues` of them and each
    /// at most `maxSize` bytes (docs/adr/0014). The value is cut before it is unescaped: with `text`
    /// a separator that a backslash precedes is part of the value, and with anything else every
    /// separator cuts. The parts are judged left to right and the first failure is the line's: a part
    /// past `maxValues` is `lineTooMany`, even one that is empty or too long; an empty part is
    /// `lineBadValue`.
    public mutating func readStrings(
        separator: Unicode.Scalar, maxValues: Int, maxSize: Int, text: Bool
    ) throws(CodecError) -> [String] {
        try beginValue()
        var values: [String] = []
        var buf: [UInt8] = []
        while true {
            let b = scan.bump()
            if b < 0 || b == ascii(separator) {
                if buf.isEmpty { throw .lineBadValue }
                values.append(try ContentLine.utf8(buf))
                if b < 0 { return values }
                buf.removeAll(keepingCapacity: true)
                // A separator opens another part, which may not pass the bound.
                if values.count >= maxValues { throw .lineTooMany }
                continue
            }
            let byte: UInt8
            if text && b == ascii("\\") {
                byte = try unescaped()
            } else if ContentLine.isControl(b) {
                throw .lineBadValue
            } else {
                byte = UInt8(b)
            }
            if buf.count == maxSize { throw .lineTooLong }
            buf.append(byte)
        }
    }

    /// The decimal the rest of the value is (an optional sign, digits) as its sign and magnitude. A
    /// `-` is read only when `allowMinus`, so an unsigned type refuses `-0` as well.
    private mutating func readDecimal(allowMinus: Bool) throws(CodecError) -> (negative: Bool, magnitude: UInt64) {
        try beginValue()
        var next = scan.bump()
        let negative = next == ascii("-")
        if negative && !allowMinus { throw .lineBadValue }
        if negative || next == ascii("+") { next = scan.bump() }
        var magnitude: UInt64 = 0
        var digits = 0
        while next >= 0 {
            if next < ascii("0") || next > ascii("9") { throw .lineBadValue }
            let d = UInt64(next - ascii("0"))
            if magnitude > (UInt64.max - d) / 10 { throw .lineBadValue }
            magnitude = magnitude * 10 + d
            digits += 1
            next = scan.bump()
        }
        if digits == 0 { throw .lineBadValue }
        return (negative, magnitude)
    }

    /// Reads the value as an unsigned integer of at most `max`.
    public mutating func readUint(max: UInt64) throws(CodecError) -> UInt64 {
        let (_, magnitude) = try readDecimal(allowMinus: false)
        if magnitude > max { throw .lineBadValue }
        return magnitude
    }

    /// Reads the value as a signed integer within `min...max`.
    public mutating func readInt(min: Int64, max: Int64) throws(CodecError) -> Int64 {
        let (negative, magnitude) = try readDecimal(allowMinus: true)
        let value: Int64
        if negative {
            if magnitude > 0x8000_0000_0000_0000 { throw .lineBadValue }
            // 2^63 is the magnitude of Int64.min, whose two's complement is itself.
            value = Int64(bitPattern: 0 &- magnitude)
        } else {
            if magnitude > UInt64(Int64.max) { throw .lineBadValue }
            value = Int64(magnitude)
        }
        if value < min || value > max { throw .lineBadValue }
        return value
    }

    /// Reads the value as `TRUE` or `FALSE`, in either case.
    public mutating func readBool() throws(CodecError) -> Bool {
        try beginValue()
        var word: [UInt8] = []
        while true {
            let b = scan.bump()
            if b < 0 { break }
            if word.count == 5 { throw .lineBadValue }
            word.append(UInt8(ContentLine.lower(b)))
        }
        switch String(decoding: word, as: UTF8.self) {
        case "true": return true
        case "false": return false
        default: throw .lineBadValue
        }
    }

    /// Reads the value as the text of a variant of an enum (docs/adr/0015): the carrier of the first
    /// row of `table` whose text it is, ASCII case-insensitively. A value no row names is
    /// `lineBadValue`, whatever its bytes are: it is no TEXT, so a backslash in it is not an escape,
    /// and nothing of it is kept past the longest text.
    public mutating func readEnum(_ table: EnumTexts) throws(CodecError) -> UInt64 {
        try beginValue()
        let longest = table.longest
        var word: [UInt8] = []
        while true {
            let b = scan.bump()
            if b < 0 { break }
            if word.count <= longest { word.append(UInt8(b)) }
        }
        return try table.carrier(of: word)
    }

    /// Reads the value of the parameter `nextParam()` stands on as the text of a variant of an enum
    /// (docs/adr/0015). The whole value is scanned before it is judged, so a line the grammar refuses
    /// is `lineMalformed` before it is `lineBadValue`; a second value is `lineBadValue`.
    public mutating func readParamEnum(_ table: EnumTexts) throws(CodecError) -> UInt64 {
        if phase != .paramValue { throw .lineMalformed }
        let longest = table.longest
        var word: [UInt8] = []
        let more = try scanParamValue { b throws(CodecError) in
            if word.count <= longest { word.append(UInt8(b)) }
        }
        paramFrom = -1
        phase = .atSeparator
        if more { throw .lineBadValue }
        return try table.carrier(of: word)
    }
}

/// One variant of an enum a content-line entry is read and written by (docs/adr/0015): the text the
/// enum document gives it and the carrier value it stands for.
public struct EnumText: Sendable {
    public let text: String
    public let value: UInt64

    public init(_ text: String, _ value: UInt64) {
        self.text = text
        self.value = value
    }
}

/// The variants of an enum in declaration order. A generated codec holds it as data, because the
/// enum's own type carries no text.
public struct EnumTexts: Sendable {
    private let rows: [EnumText]

    public init(_ rows: [EnumText]) {
        self.rows = rows
    }

    var longest: Int { rows.map { $0.text.utf8.count }.max() ?? 0 }

    /// The carrier of the first variant whose text `value` is, compared ASCII case-insensitively:
    /// only the 26 letters fold, so a character Unicode folds to one of them (U+017F, U+0131) is not
    /// that letter. A value no variant names is `lineBadValue`.
    func carrier(of value: [UInt8]) throws(CodecError) -> UInt64 {
        for row in rows {
            let text = Array(row.text.utf8)
            if text.count != value.count { continue }
            var same = true
            for i in value.indices where ContentLine.lower(Int(text[i])) != ContentLine.lower(Int(value[i])) {
                same = false
                break
            }
            if same { return row.value }
        }
        throw .lineBadValue
    }

    /// The text of the first variant with `carrier`, as the enum declares it; a carrier of an open
    /// enum that no variant declares has none, which the caller answers `lineBadValue`.
    func text(of carrier: UInt64) -> String? {
        rows.first { $0.value == carrier }?.text
    }
}

// MARK: Writing

/// A writer of the lines of one component into the frame it holds.
///
/// Write `begin()`; then a property by `property(_:)`, each present parameter by `param`, and the
/// value by one of the value methods, which ends the line; then `finish()`. Each throws the
/// `CodecError` that refused it. The frame is `bytes`, which the generated code writes to its sink
/// once the whole component was written.
public struct ContentLineWriter {
    /// The lines written so far.
    public private(set) var bytes: [UInt8] = []
    private let component: String
    /// Octets already on the current physical line.
    private var column = 0

    public init(component: String) {
        self.component = component
    }

    /// Writes `BEGIN:<component>`.
    public mutating func begin() {
        bytes += Array("BEGIN:\(component)\r\n".utf8)
    }

    /// Writes `END:<component>`.
    public mutating func finish() {
        bytes += Array("END:\(component)\r\n".utf8)
    }

    /// Writes one unit (a character, or an escape) on the current line, after cutting the line if it
    /// would pass `ContentLine.foldWidth` octets.
    private mutating func unit(_ unit: [UInt8]) {
        if column + unit.count > ContentLine.foldWidth {
            bytes += [cr, lf, space]
            column = 1
        }
        bytes += unit
        column += unit.count
    }

    private mutating func unit(_ c: Unicode.Scalar) {
        unit([UInt8(c.value)])
    }

    /// The units of ASCII `text`: one octet each.
    private mutating func asciiUnits(_ text: String) {
        for b in text.utf8 { unit([b]) }
    }

    /// The units of UTF-8 `value`: one character each, escaped when `text`.
    private mutating func valueUnits(_ value: [UInt8], text: Bool) {
        var i = 0
        while i < value.count {
            let lead = Int(value[i])
            let length: Int
            switch lead {
            case ..<0x80: length = 1
            case ..<0xE0: length = 2
            case ..<0xF0: length = 3
            default: length = 4
            }
            let size = Swift.min(length, value.count - i)
            var escaped: [UInt8]?
            if text && size == 1 {
                switch lead {
                case ascii("\\"): escaped = [UInt8(ascii("\\")), UInt8(ascii("\\"))]
                case ascii(";"): escaped = [UInt8(ascii("\\")), UInt8(ascii(";"))]
                case ascii(","): escaped = [UInt8(ascii("\\")), UInt8(ascii(","))]
                case 0x0A: escaped = [UInt8(ascii("\\")), UInt8(ascii("n"))]
                default: escaped = nil
                }
            }
            unit(escaped ?? Array(value[i..<(i + size)]))
            i += size
        }
    }

    /// Starts a property's line with its name.
    public mutating func property(_ name: String) {
        column = 0
        asciiUnits(name)
    }

    /// Writes `;<name>=<value>`, quoting the value when it holds `:`, `;` or `,`. A value past
    /// `maxSize` bytes is `lineTooLong`; one with a control character or a `"` is `lineBadValue`.
    public mutating func param(_ name: String, _ value: String, maxSize: Int) throws(CodecError) {
        let encoded = Array(value.utf8)
        if encoded.count > maxSize { throw .lineTooLong }
        if encoded.contains(where: { Int($0) == ascii("\"") || ContentLine.isControl(Int($0)) }) {
            throw .lineBadValue
        }
        let quoted = encoded.contains { Int($0) == ascii(":") || Int($0) == ascii(";") || Int($0) == ascii(",") }
        unit(";")
        asciiUnits(name)
        unit("=")
        if quoted { unit("\"") }
        valueUnits(encoded, text: false)
        if quoted { unit("\"") }
    }

    private mutating func endLine() {
        column = 0
        bytes += [cr, lf]
    }

    /// Writes `:<value>` and ends the line. With `text`, `\`, `;`, `,` and a line feed are written
    /// as escapes. A value past `maxSize` bytes is `lineTooLong`; one with a control character (but a
    /// TEXT's line feed) is `lineBadValue`.
    public mutating func string(_ value: String, text: Bool, maxSize: Int) throws(CodecError) {
        let encoded = Array(value.utf8)
        if encoded.count > maxSize { throw .lineTooLong }
        if encoded.contains(where: { ContentLine.isControl(Int($0)) && !(text && $0 == lf) }) {
            throw .lineBadValue
        }
        unit(":")
        valueUnits(encoded, text: text)
        endLine()
    }

    /// Writes `:<value>{separator}<value>...` and ends the line (docs/adr/0014). No values is
    /// `lineRequiredMissing` and more than `maxValues` is `lineTooMany`. A value past `maxSize`
    /// bytes is `lineTooLong`; one with a control character (but a TEXT's line feed), an empty one,
    /// and one that is not a TEXT and holds the separator are `lineBadValue`, because a reader would
    /// cut or refuse them. Every value is held before any of the line is written.
    public mutating func strings(
        _ values: [String], separator: Unicode.Scalar, text: Bool, maxSize: Int, maxValues: Int
    ) throws(CodecError) {
        if values.isEmpty { throw .lineRequiredMissing }
        if values.count > maxValues { throw .lineTooMany }
        let encoded = values.map { Array($0.utf8) }
        for value in encoded {
            if value.count > maxSize { throw .lineTooLong }
            if value.contains(where: { ContentLine.isControl(Int($0)) && !(text && $0 == lf) }) {
                throw .lineBadValue
            }
            if value.isEmpty || (!text && value.contains(where: { Int($0) == ascii(separator) })) {
                throw .lineBadValue
            }
        }
        unit(":")
        for (index, value) in encoded.enumerated() {
            if index > 0 { unit(separator) }
            valueUnits(value, text: text)
        }
        endLine()
    }

    private mutating func digits(_ text: String) {
        unit(":")
        asciiUnits(text)
        endLine()
    }

    /// Writes `:<value>` as decimal digits and ends the line.
    public mutating func uint(_ value: UInt64) {
        digits(String(value))
    }

    /// Writes `:<value>` as decimal digits, `-` first when negative, and ends the line.
    public mutating func int(_ value: Int64) {
        digits(String(value))
    }

    /// Writes `:TRUE` or `:FALSE` and ends the line.
    public mutating func boolean(_ value: Bool) {
        digits(value ? "TRUE" : "FALSE")
    }

    /// Writes `;<name>=<text>` for the variant with `carrier`. A text is letters, digits and hyphens,
    /// so it is never quoted (docs/adr/0015). A carrier no variant declares has no text and is
    /// `lineBadValue`.
    public mutating func enumParam(_ name: String, _ table: EnumTexts, carrier: UInt64) throws(CodecError) {
        guard let text = table.text(of: carrier) else { throw .lineBadValue }
        unit(";")
        asciiUnits(name)
        unit("=")
        asciiUnits(text)
    }

    /// Writes `:<text>` for the variant with `carrier` and ends the line (docs/adr/0015). A carrier no
    /// variant declares has no text and is `lineBadValue`.
    public mutating func enumValue(_ table: EnumTexts, carrier: UInt64) throws(CodecError) {
        guard let text = table.text(of: carrier) else { throw .lineBadValue }
        digits(text)
    }
}
