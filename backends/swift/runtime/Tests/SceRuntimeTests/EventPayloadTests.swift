// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// A field a later revision of an event schema added is read out of the schema's
// defaults where a delivery lacks it, and a field of the first revision is still
// required (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
// `static_schema_revision` holds the generated machines to the same answers; the
// cases here pin the reader itself.

private let revisedDefaults = #"{"layout":"week","count":"7","urgent":true}"#

@Test func aFieldTheDeliveryLacksIsReadOutOfTheDefaults() throws {
    let fields = try PayloadFields.decode(#"{"level": 4}"#, defaults: revisedDefaults)
    #expect(try fields.unsigned("level") as UInt8 == 4)
    #expect(try fields.text("layout") == "week")
    #expect(try fields.unsigned64("count") == 7)
    #expect(try fields.truth("urgent") == true)
}

@Test func aFieldTheDeliveryCarriesIsReadAsItCame() throws {
    let fields = try PayloadFields.decode(
        #"{"level": 4, "layout": "day", "urgent": false, "source": {"a": [1]}}"#,
        defaults: revisedDefaults
    )
    #expect(try fields.text("layout") == "day")
    #expect(try fields.truth("urgent") == false)
    // A value of another type is refused, default or not.
    let wrong = try PayloadFields.decode(#"{"level": 4, "urgent": 1}"#, defaults: revisedDefaults)
    #expect(throws: PayloadRefusal.self) { try wrong.truth("urgent") }
}

@Test func aFieldOfTheFirstRevisionIsStillRequired() throws {
    let fields = try PayloadFields.decode(#"{"layout": "day"}"#, defaults: revisedDefaults)
    let refusal = #expect(throws: PayloadRefusal.self) { try fields.unsigned("level") as UInt8 }
    #expect(refusal?.reason.contains("'level'") == true)
    // No data at all is a delivery that names no field.
    let none = try PayloadFields.decode("", defaults: revisedDefaults)
    #expect(throws: PayloadRefusal.self) { try none.unsigned("level") as UInt8 }
    #expect(try none.truth("urgent") == true)
    // A bare value is still no object of named fields.
    #expect(throws: PayloadRefusal.self) { try PayloadFields.decode("5", defaults: revisedDefaults) }
}

@Test func aWholeNumberIsRefusedPastItsWidthAndNeverWrapped() throws {
    let fields = try PayloadFields.decode(#"{"small": 300, "negative": -1, "real": 1.5, "flag": true}"#)
    #expect(throws: PayloadRefusal.self) { try fields.unsigned("small") as UInt8 }
    #expect(try fields.unsigned("small") as UInt16 == 300)
    #expect(throws: PayloadRefusal.self) { try fields.unsigned("negative") as UInt32 }
    #expect(try fields.signed("negative") as Int8 == -1)
    #expect(throws: PayloadRefusal.self) { try fields.signed("real") as Int32 }
    // A truth value is not a number.
    #expect(throws: PayloadRefusal.self) { try fields.signed("flag") as Int32 }
    #expect(try fields.float64("real") == 1.5)
}

@Test func aSixtyFourBitFieldCrossesAsTheTextOfItsDigits() throws {
    let fields = try PayloadFields.decode(
        #"{"big": "18446744073709551615", "neg": "-9223372036854775808", "exact": 9007199254740991, "past": 9007199254740992, "lead": "007"}"#
    )
    #expect(try fields.unsigned64("big") == UInt64.max)
    #expect(try fields.signed64("neg") == Int64.min)
    #expect(try fields.signed64("exact") == 9_007_199_254_740_991)
    // A number past 2^53 - 1 is not exact everywhere and is refused.
    #expect(throws: PayloadRefusal.self) { try fields.signed64("past") }
    // Digits only, no leading zero.
    #expect(throws: PayloadRefusal.self) { try fields.unsigned64("lead") }
    // A negative text is no unsigned value.
    #expect(throws: PayloadRefusal.self) { try fields.unsigned64("neg") }
}

@Test func aSingleFractionIsTheBinary32NearestTheDoubleAndNotRoundedFromTheText() throws {
    let fields = try PayloadFields.decode(#"{"ok": 0.1, "huge": 1e39}"#)
    #expect(try fields.float32("ok") == Float(0.1))
    #expect(throws: PayloadRefusal.self) { try fields.float32("huge") }
    #expect(try fields.float64("huge") == 1e39)
}

@Test func aByteStringCrossesAsItsLatin1TextAndComesBackTheSame() throws {
    let all = (0...255).map { UInt8($0) }
    let json = payloadJSON(.bytes("b", all))
    let fields = try PayloadFields.decode(json)
    #expect(try fields.bytes("b") == all)
    #expect(throws: PayloadRefusal.self) { try fields.bytes("b", capacity: 255) }
    let wide = try PayloadFields.decode("{\"w\":\"\u{100}\"}")
    #expect(throws: PayloadRefusal.self) { try wide.bytes("w") }
}

@Test func anOptionalFieldReadsNullAsAbsentAndAValueAsPresent() throws {
    let fields = try PayloadFields.decode(#"{"id": 1, "due": null, "title": "soon"}"#)
    #expect(try fields.optional("due") { try $0.signed64("due") } == nil)
    #expect(try fields.optional("title") { try $0.text("title") } == "soon")
    // A present value is held to the field's type, as a field of T is.
    #expect(throws: PayloadRefusal.self) { try fields.optional("title") { try $0.signed64("title") } }
}

@Test func anOptionalFieldWhoseKeyTheDeliveryLacksIsRefused() throws {
    let fields = try PayloadFields.decode(#"{"id": 3, "title": "gone"}"#)
    let refusal = #expect(throws: PayloadRefusal.self) { try fields.optional("due") { try $0.signed64("due") } }
    #expect(refusal?.reason.contains("'due'") == true)
    // A later revision's default of null is an absent value.
    let later = try PayloadFields.decode(#"{"id": 3}"#, defaults: #"{"due":null}"#)
    #expect(try later.optional("due") { try $0.signed64("due") } == nil)
}

@Test func aFieldOfTRefusesNullAsAnOlderReceiverOfAWidenedFieldDoes() throws {
    let fields = try PayloadFields.decode(#"{"id": 1, "due": null}"#)
    let refusal = #expect(throws: PayloadRefusal.self) { try fields.signed64("due") }
    #expect(refusal?.reason.contains("'due'") == true)
    #expect(try fields.optional("due") { try $0.signed64("due") } == nil)
}

@Test func anAbsentValueIsWrittenAsNullAndAPresentOneAsItsWriterSpellsIt() {
    let digits = { (v: Int64) in "\"\(v)\"" }
    #expect(optionalJSON(nil, write: digits) == "null")
    #expect(optionalJSON(Int64(12), write: digits) == "\"12\"")
    #expect(optionalJSON("soon", write: quoteJSON) == "\"soon\"")
}

// An inject seam's payload is written in the order its schema declares the fields,
// as every other engine writes it, and not in name order.
@Test func aPayloadKeepsTheOrderItsSchemaDeclares() {
    let got = payloadJSON(.text("payload", "q\"\u{1}"), .integer("offset", 7), .truth("flag", true))
    #expect(got == #"{"payload":"q\"\u0001","offset":7,"flag":true}"#)
    #expect(payloadJSON() == "{}")
    #expect(payloadJSON(.real("x", 5.0), .real("y", .nan), .signed64("z", -3)) == #"{"x":5,"y":null,"z":"-3"}"#)
}

// A value a `<param>` computes crosses as the JSON the pair of the same type writes:
// a 64-bit integer as the text of its digits, a number as a number.
@Test func aWireValueIsWrittenAsItsPairIs() {
    let pairs = [
        PayloadPair("n", .integer(-4)),
        PayloadPair("big", .unsigned64(UInt64.max)),
        PayloadPair("wide", .signed64(Int64.min)),
        PayloadPair("r", .real(2.5)),
        PayloadPair("t", .truth(false)),
        PayloadPair("s", .text("a\"b")),
        PayloadPair("b", .bytes([0x61, 0xFF])),
        PayloadPair("absent", .null),
    ]
    #expect(
        payloadJSON(pairs)
            == #"{"n":-4,"big":"18446744073709551615","wide":"-9223372036854775808","r":2.5,"t":false,"s":"a\"b","b":"aÿ","absent":null}"#
    )
    #expect(WireValue.text("x").json == "\"x\"")
}

// A form carries a value as ECMAScript's `String(value)`: a text without the quotes JSON
// wraps it in, a number as the shortest text that reads back, absence as the empty text.

@Test func aFormCarriesEachValueAsTheTextItSpells() {
    #expect(WireValue.text("busy").formText == "busy")
    #expect(WireValue.integer(-5).formText == "-5")
    #expect(WireValue.signed64(-9_223_372_036_854_775_808).formText == "-9223372036854775808")
    #expect(WireValue.unsigned64(18_446_744_073_709_551_615).formText == "18446744073709551615")
    #expect(WireValue.real(1.5).formText == "1.5")
    #expect(WireValue.real(8).formText == "8")
    #expect(WireValue.truth(true).formText == "true")
    #expect(WireValue.bytes([0x61, 0xFF]).formText == "aÿ")
}

@Test func aFormCarriesAnAbsentValueAsTheEmptyTextAndANonFiniteRealByItsName() {
    #expect(WireValue.null.formText.isEmpty)
    #expect(WireValue.real(.nan).formText == "NaN")
    #expect(WireValue.real(.infinity).formText == "Infinity")
    #expect(WireValue.real(-.infinity).formText == "-Infinity")
}

@Test func aFormParameterThatRepeatsKeepsEveryValueInDocumentOrder() {
    var form: [String: [String]] = [:]
    appendFormParam(&form, "tag", .text("a"))
    appendFormParam(&form, "n", .integer(2))
    appendFormParam(&form, "tag", .text("b"))
    #expect(form == ["tag": ["a", "b"], "n": ["2"]])
}
