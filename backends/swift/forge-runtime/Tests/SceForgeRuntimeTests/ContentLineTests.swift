// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// The content lines a codec reads and writes (SCE_FORGE.md §4.6.4, docs/adr/0010, 0014 and 0015),
// held to the rules the other runtimes' own tests hold (`ContentLineCodecTest.kt`, `content_line_test.go`):
// a component is walked property by property with its nested components skipped, a line is read
// through its folds, a TEXT value is unescaped and any other value is not, each failure is the error of
// the rule that refused it, and a write folds at 75 octets and writes what a read takes back.

private func bytes(_ text: String) -> [UInt8] { Array(text.utf8) }

private func reader(_ input: String, component: String = "V") throws -> ContentLineReader {
    try ContentLineReader.begin(bytes(input), component: component)
}

/// The value of the only property of `V` as a string.
private func stringOf(_ line: String, maxSize: Int = 64, text: Bool = true) throws -> String {
    var component = try reader("BEGIN:V\r\n\(line)\r\nEND:V\r\n")
    var property = try #require(try component.next())
    return try property.readString(maxSize: maxSize, text: text)
}

private func property(_ line: String) throws -> ContentLineProperty {
    var component = try reader("BEGIN:V\r\n\(line)\r\nEND:V\r\n")
    return try #require(try component.next())
}

// MARK: Reading a component

@Test func aComponentIsWalkedPropertyByPropertyToItsEnd() throws {
    let input = "BEGIN:V\r\nA:1\r\nB;P=x:2\r\nEND:V\r\n"
    var component = try reader(input)
    var first = try #require(try component.next())
    #expect(first.isNamed("a"))
    #expect(try first.readUint(max: 255) == 1)
    var second = try #require(try component.next())
    #expect(second.isNamed("B"))
    #expect(try second.nextParam())
    #expect(second.paramNamed("p"))
    #expect(try second.readParamString(maxSize: 8) == "x")
    #expect(try second.nextParam() == false)
    #expect(try component.next() == nil)
    #expect(component.consumed == input.utf8.count)
}

@Test func lineEndsAreCrLfOrLfAndNamesAreNotCaseSensitive() throws {
    var component = try reader("begin:v\nname:x\nend:V\n")
    var name = try #require(try component.next())
    #expect(name.isNamed("NAME"))
    #expect(try name.readString(maxSize: 8, text: false) == "x")
    #expect(try component.next() == nil)
}

@Test func linesBeforeTheBeginAreSkippedAndANestedComponentIsPassedThrough() throws {
    var component = try reader("X:1\r\nBEGIN:V\r\nBEGIN:ALARM\r\nA:no\r\nEND:ALARM\r\nB:yes\r\nEND:V\r\n")
    let only = try #require(try component.next())
    #expect(only.isNamed("B"))
    #expect(try component.next() == nil)
}

@Test func aFoldedLineIsReadAsIfItWereWhole() throws {
    #expect(try stringOf("A:ab\r\n cd\r\n\tef", text: false) == "abcdef")
    // The cut may fall inside a name.
    var component = try reader("BEGIN:V\r\nDESC\r\n RIPTION:x\r\nEND:V\r\n")
    #expect(try #require(try component.next()).isNamed("DESCRIPTION"))
}

@Test func anInputThatEndsBeforeTheEndIsNeedMoreBytes() throws {
    #expect(throws: CodecError.needMoreBytes) { try reader("A:1\r\n") }
    var open = try reader("BEGIN:V\r\nA:1\r\n")
    _ = try open.next()
    #expect(throws: CodecError.needMoreBytes) { try open.next() }
    var cut = try reader("BEGIN:V\r\nA:1")
    #expect(throws: CodecError.needMoreBytes) { try cut.next() }
}

@Test func aLineWithNoNameOrTheWrongEndIsMalformed() throws {
    var nameless = try reader("BEGIN:V\r\n:x\r\nEND:V\r\n")
    #expect(throws: CodecError.lineMalformed) { try nameless.next() }
    var wrong = try reader("BEGIN:V\r\nEND:W\r\nEND:V\r\n")
    #expect(throws: CodecError.lineMalformed) { try wrong.next() }
}

// MARK: Reading a value

@Test func aTextIsUnescapedAndAnyOtherStringIsNot() throws {
    #expect(try stringOf("A:a\\nb\\;c\\,d\\\\e") == "a\nb;c,d\\e")
    #expect(try stringOf("A:a\\Nb") == "a\nb")
    #expect(try stringOf("A:a\\nb;c,d", text: false) == "a\\nb;c,d")
    #expect(throws: CodecError.lineBadEscape) { try stringOf("A:a\\xb") }
}

@Test func aValueIsHeldToItsBoundAndToTheCharactersAValueMayHold() throws {
    #expect(try stringOf("A:abcd", maxSize: 4) == "abcd")
    #expect(throws: CodecError.lineTooLong) { try stringOf("A:abcde", maxSize: 4) }
    #expect(try stringOf("A:a\tb") == "a\tb")
    #expect(throws: CodecError.lineBadValue) { try stringOf("A:a\u{1}b") }
    #expect(throws: CodecError.lineBadValue) { try stringOf("A:a\u{7F}b") }
}

@Test func aValueThatIsNotUtf8IsBadValue() throws {
    var component = try ContentLineReader.begin([0x42, 0x45, 0x47, 0x49, 0x4E, 0x3A, 0x56, 0x0A, 0x41, 0x3A, 0xFF, 0x0A, 0x45, 0x4E, 0x44, 0x3A, 0x56, 0x0A], component: "V")
    var property = try #require(try component.next())
    #expect(throws: CodecError.lineBadValue) { try property.readString(maxSize: 8, text: false) }
}

@Test func aListIsCutAtItsSeparatorAndJudgedLeftToRight() throws {
    var line = try property("A:a,b\\,c,d")
    #expect(try line.readStrings(separator: ",", maxValues: 3, maxSize: 8, text: true) == ["a", "b,c", "d"])
    var many = try property("A:a,b,c")
    #expect(throws: CodecError.lineTooMany) { try many.readStrings(separator: ",", maxValues: 2, maxSize: 8, text: true) }
    var empty = try property("A:a,,c")
    #expect(throws: CodecError.lineBadValue) { try empty.readStrings(separator: ",", maxValues: 3, maxSize: 8, text: true) }
    var long = try property("A:a,bcdef")
    #expect(throws: CodecError.lineTooLong) { try long.readStrings(separator: ",", maxValues: 3, maxSize: 4, text: true) }
    // A part past the bound is `lineTooMany` even when it is empty.
    var past = try property("A:a,b,")
    #expect(throws: CodecError.lineTooMany) { try past.readStrings(separator: ",", maxValues: 2, maxSize: 8, text: true) }
}

@Test func anUnsignedIntegerIsDigitsWithinItsMaximum() throws {
    var zero = try property("A:0")
    #expect(try zero.readUint(max: 255) == 0)
    var plus = try property("A:+255")
    #expect(try plus.readUint(max: 255) == 255)
    var large = try property("A:18446744073709551615")
    #expect(try large.readUint(max: UInt64.max) == UInt64.max)
    var past = try property("A:256")
    #expect(throws: CodecError.lineBadValue) { try past.readUint(max: 255) }
    var overflow = try property("A:18446744073709551616")
    #expect(throws: CodecError.lineBadValue) { try overflow.readUint(max: UInt64.max) }
    var minus = try property("A:-0")
    #expect(throws: CodecError.lineBadValue) { try minus.readUint(max: 255) }
    var none = try property("A:")
    #expect(throws: CodecError.lineBadValue) { try none.readUint(max: 255) }
    var letter = try property("A:1a")
    #expect(throws: CodecError.lineBadValue) { try letter.readUint(max: 255) }
}

@Test func aSignedIntegerIsHeldToItsRange() throws {
    var low = try property("A:-9223372036854775808")
    #expect(try low.readInt(min: Int64.min, max: Int64.max) == Int64.min)
    var high = try property("A:9223372036854775807")
    #expect(try high.readInt(min: Int64.min, max: Int64.max) == Int64.max)
    var below = try property("A:-9223372036854775809")
    #expect(throws: CodecError.lineBadValue) { try below.readInt(min: Int64.min, max: Int64.max) }
    var above = try property("A:9223372036854775808")
    #expect(throws: CodecError.lineBadValue) { try above.readInt(min: Int64.min, max: Int64.max) }
    var narrow = try property("A:-129")
    #expect(throws: CodecError.lineBadValue) { try narrow.readInt(min: -128, max: 127) }
}

@Test func aTruthValueIsTrueOrFalseInEitherCase() throws {
    var upper = try property("A:TRUE")
    #expect(try upper.readBool())
    var mixed = try property("A:fAlSe")
    #expect(try mixed.readBool() == false)
    var other = try property("A:yes")
    #expect(throws: CodecError.lineBadValue) { try other.readBool() }
    var long = try property("A:falsey")
    #expect(throws: CodecError.lineBadValue) { try long.readBool() }
}

private let statuses = EnumTexts([EnumText("NEEDS-ACTION", 0), EnumText("accepted", 1), EnumText("IN-PROCESS", 2)])

@Test func anEnumIsTheCarrierOfTheVariantWhoseTextTheValueIsInAnyCase() throws {
    var first = try property("A:needs-action")
    #expect(try first.readEnum(statuses) == 0)
    var second = try property("A:ACCEPTED")
    #expect(try second.readEnum(statuses) == 1)
    var unknown = try property("A:declined")
    #expect(throws: CodecError.lineBadValue) { try unknown.readEnum(statuses) }
    // Only the 26 letters fold: U+017F is not an `s`.
    var folded = try property("A:in-proce\u{017F}\u{017F}")
    #expect(throws: CodecError.lineBadValue) { try folded.readEnum(statuses) }
}

// MARK: Reading parameters

@Test func aParameterIsReadByItsNameAndOneNotReadIsSkipped() throws {
    var line = try property("A;SKIP=1;ROLE=chair;MORE=\"q;r:s\":value")
    #expect(try line.nextParam())
    #expect(line.paramNamed("skip"))
    #expect(try line.nextParam())
    #expect(line.paramNamed("ROLE"))
    #expect(try line.readParamString(maxSize: 8) == "chair")
    #expect(try line.nextParam())
    #expect(line.paramNamed("more"))
    #expect(try line.readParamString(maxSize: 8) == "q;r:s")
    #expect(try line.nextParam() == false)
    #expect(try line.readString(maxSize: 8, text: false) == "value")
}

@Test func aPropertyReadByValueAloneSkipsItsParameters() throws {
    var line = try property("A;P=1;Q=2:value")
    #expect(try line.readString(maxSize: 8, text: false) == "value")
}

@Test func aParameterIsHeldToOneValueAndToItsBound() throws {
    var two = try property("A;P=1,2:v")
    _ = try two.nextParam()
    #expect(throws: CodecError.lineBadValue) { try two.readParamString(maxSize: 8) }
    var long = try property("A;P=abcdef:v")
    _ = try long.nextParam()
    #expect(throws: CodecError.lineTooLong) { try long.readParamString(maxSize: 4) }
    var stray = try property("A;P=a\"b:v")
    _ = try stray.nextParam()
    #expect(throws: CodecError.lineMalformed) { try stray.readParamString(maxSize: 8) }
    var bare = try property("A;P:v")
    #expect(throws: CodecError.lineMalformed) { try bare.nextParam() }
}

@Test func aParameterMayBeAnEnum() throws {
    var line = try property("A;PARTSTAT=Accepted:v")
    _ = try line.nextParam()
    #expect(try line.readParamEnum(statuses) == 1)
    var two = try property("A;PARTSTAT=accepted,accepted:v")
    _ = try two.nextParam()
    #expect(throws: CodecError.lineBadValue) { try two.readParamEnum(statuses) }
}

// MARK: Writing

private func written(_ body: (inout ContentLineWriter) throws(CodecError) -> Void) throws(CodecError) -> String {
    var writer = ContentLineWriter(component: "V")
    writer.begin()
    try body(&writer)
    writer.finish()
    return String(decoding: writer.bytes, as: UTF8.self)
}

@Test func aComponentIsWrittenWithItsPropertiesParametersAndValues() throws {
    let text = try written { writer throws(CodecError) in
        writer.property("A")
        try writer.param("P", "x", maxSize: 8)
        try writer.string("hello", text: true, maxSize: 16)
        writer.property("N")
        writer.uint(42)
        writer.property("I")
        writer.int(-7)
        writer.property("B")
        writer.boolean(true)
    }
    #expect(text == "BEGIN:V\r\nA;P=x:hello\r\nN:42\r\nI:-7\r\nB:TRUE\r\nEND:V\r\n")
}

@Test func aTextIsEscapedAndAParameterIsQuotedWhenItHoldsASeparator() throws {
    let text = try written { writer throws(CodecError) in
        writer.property("A")
        try writer.param("P", "a:b", maxSize: 8)
        try writer.string("a\nb;c,d\\e", text: true, maxSize: 32)
    }
    #expect(text == "BEGIN:V\r\nA;P=\"a:b\":a\\nb\\;c\\,d\\\\e\r\nEND:V\r\n")
}

@Test func aLineIsFoldedAtSeventyFiveOctets() throws {
    let text = try written { writer throws(CodecError) in
        writer.property("A")
        try writer.string(String(repeating: "x", count: 100), text: false, maxSize: ContentLine.unbounded)
    }
    let first = "A:" + String(repeating: "x", count: 73)
    #expect(text == "BEGIN:V\r\n\(first)\r\n \(String(repeating: "x", count: 27))\r\nEND:V\r\n")
    // A character is never cut: a two-octet one that would pass the width starts the next line.
    let wide = try written { writer throws(CodecError) in
        writer.property("A")
        try writer.string(String(repeating: "x", count: 72) + "\u{00E9}", text: false, maxSize: 200)
    }
    #expect(wide == "BEGIN:V\r\nA:\(String(repeating: "x", count: 72))\r\n \u{00E9}\r\nEND:V\r\n")
}

@Test func whatIsWrittenIsReadBack() throws {
    let value = String(repeating: "ab;c,d\n", count: 40)
    var writer = ContentLineWriter(component: "V")
    writer.begin()
    writer.property("NOTE")
    try writer.string(value, text: true, maxSize: 1000)
    writer.property("LIST")
    try writer.strings(["a,b", "c;d", "e"], separator: ",", text: true, maxSize: 8, maxValues: 4)
    writer.finish()
    var component = try ContentLineReader.begin(writer.bytes, component: "V")
    var note = try #require(try component.next())
    #expect(note.isNamed("NOTE"))
    #expect(try note.readString(maxSize: 1000, text: true) == value)
    var list = try #require(try component.next())
    #expect(try list.readStrings(separator: ",", maxValues: 4, maxSize: 8, text: true) == ["a,b", "c;d", "e"])
    #expect(try component.next() == nil)
    #expect(component.consumed == writer.bytes.count)
}

@Test func aValueTheReaderWouldRefuseIsRefusedByTheWriter() throws {
    var writer = ContentLineWriter(component: "V")
    writer.property("A")
    #expect(throws: CodecError.lineTooLong) { try writer.string("abcde", text: true, maxSize: 4) }
    #expect(throws: CodecError.lineBadValue) { try writer.string("a\u{1}", text: true, maxSize: 8) }
    #expect(throws: CodecError.lineBadValue) { try writer.string("a\nb", text: false, maxSize: 8) }
    #expect(throws: CodecError.lineBadValue) { try writer.param("P", "a\"b", maxSize: 8) }
    #expect(throws: CodecError.lineTooLong) { try writer.param("P", "abcde", maxSize: 4) }
    #expect(throws: CodecError.lineRequiredMissing) {
        try writer.strings([], separator: ",", text: true, maxSize: 8, maxValues: 2)
    }
    #expect(throws: CodecError.lineTooMany) {
        try writer.strings(["a", "b", "c"], separator: ",", text: true, maxSize: 8, maxValues: 2)
    }
    #expect(throws: CodecError.lineBadValue) {
        try writer.strings(["a", ""], separator: ",", text: true, maxSize: 8, maxValues: 2)
    }
    #expect(throws: CodecError.lineBadValue) {
        try writer.strings(["a,b"], separator: ",", text: false, maxSize: 8, maxValues: 2)
    }
}

@Test func anEnumIsWrittenAsTheTextOfItsVariant() throws {
    let text = try written { writer throws(CodecError) in
        writer.property("A")
        try writer.enumParam("PARTSTAT", statuses, carrier: 1)
        try writer.enumValue(statuses, carrier: 2)
    }
    #expect(text == "BEGIN:V\r\nA;PARTSTAT=accepted:IN-PROCESS\r\nEND:V\r\n")
    var writer = ContentLineWriter(component: "V")
    #expect(throws: CodecError.lineBadValue) { try writer.enumValue(statuses, carrier: 9) }
    #expect(throws: CodecError.lineBadValue) { try writer.enumParam("P", statuses, carrier: 9) }
}
