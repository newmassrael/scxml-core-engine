// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json, read here where they are, so this writer is
// measured against the same table as every other engine's.

@Test func aStringIsWrittenInTheOneFormEveryEngineWrites() throws {
    let cases = try sharedCases("json_text/string_escape.json")
    #expect(cases.count >= 8, "the table lost cases: \(cases.count)")
    for c in cases {
        let name = try #require(c["name"] as? String)
        let text = try #require(c["text"] as? String)
        let escaped = try #require(c["escaped"] as? String)
        #expect(escapeJSONString(text) == escaped, "\(name)")
        #expect(quoteJSON(text) == "\"" + escaped + "\"", "\(name)")
    }
}

/// `value` spelled through the typed payload writers, one call per constructor, as a
/// generated raise spells a value of its type: an object as a record, an array as a
/// list, a text quoted, a number as written.
private func typedJSON(_ value: JSONValue) -> String {
    switch value {
    case .object(let members):
        return recordJSON(members.map { PayloadPair.json($0.key, rawJSON: typedJSON($0.value)) })
    case .array(let items):
        return listJSON(items, typedJSON)
    case .text(let text):
        return quoteJSON(text)
    case .number(let number):
        return number
    case .bool(let truth):
        return truth ? "true" : "false"
    case .null:
        return "null"
    }
}

// The members of a JSON object written into `_event.data` come in one order on every
// engine (ARCHITECTURE.md, "JSON Object Key Order"). The typed writers a generated
// raise spells a record and a list with (docs/adr/0086, section 5.3) are held to
// tests/json_text/object_key_order.json, at every depth. A case whose name repeats is
// a `<send>`'s list of params, which only the parameter writer collects: a record
// holds each field once.
@Test func aRecordAndAListAreWrittenInTheOneOrderEveryEngineWrites() throws {
    let table = try parseJSON(try sharedTableText("json_text/object_key_order.json"))
    guard case .array(let cases)? = table.member("cases") else {
        Issue.record("the table has cases")
        return
    }
    var written = 0
    for c in cases {
        guard case .text(let name)? = c.member("name"), case .text(let data)? = c.member("data"),
            case .array(let params)? = c.member("params")
        else {
            Issue.record("a case has a name, params and data")
            return
        }
        var members: [PayloadPair] = []
        var names = Set<String>()
        for pair in params {
            guard case .array(let parts) = pair, parts.count == 2, case .text(let id) = parts[0] else {
                Issue.record("\(name): a param is a [name, value] pair")
                return
            }
            names.insert(id)
            members.append(PayloadPair.json(id, rawJSON: typedJSON(parts[1])))
        }
        if names.count < members.count {
            continue
        }
        #expect(recordJSON(members) == data, "\(name)")
        written += 1
    }
    #expect(written >= 10, "the table lost the cases a record is written for: \(written)")
}

@Test func anObjectThatRepeatsAFieldNameIsRefused() {
    for text in [#"{"a":1,"a":2}"#, #"{"outer":{"a":1,"b":2,"a":3}}"#, #"[{"a":1,"a":1}]"#] {
        do {
            _ = try parseJSON(text)
            Issue.record("a repeated field name was accepted: \(text)")
        } catch {
            #expect("\(error)".contains("appears twice"), "\(error)")
        }
    }
    #expect((try? parseJSON(#"{"a":1,"b":{"a":2}}"#)) != nil, "the same name at another level is fine")
}

@Test func aValueWrittenIsReadBackUnchanged() throws {
    let text = #"{"a":[1,-2.5e3,"x\ny"],"b":{"c":true,"d":null},"e":18446744073709551615}"#
    #expect(writeJSON(try parseJSON(text)) == text)
}

@Test func aWholeNumberKeepsItsSpelling() throws {
    // No Double stands between the text and the reader who picks a width.
    #expect(try parseJSON("18446744073709551615") == .number("18446744073709551615"))
}

@Test func anObjectKeepsTheOrderItsMembersWereWritten() throws {
    let value = try parseJSON(#"{"b":1,"a":2,"c":3}"#)
    guard case .object(let members) = value else {
        Issue.record("an object")
        return
    }
    #expect(members.map(\.key) == ["b", "a", "c"])
}

@Test func trailingTextAndBrokenGrammarAreRefused() {
    #expect(throws: JSONError.trailing) { try parseJSON("1 2") }
    #expect(throws: JSONError.self) { try parseJSON(#"{"a" 1}"#) }
    #expect(throws: JSONError.self) { try parseJSON("[1,") }
    #expect(throws: JSONError.self) { try parseJSON("") }
    #expect(throws: JSONError.self) { try parseJSON("-") }
}

@Test func aSurrogatePairIsOneCharacterAndALoneHalfIsTheReplacementCharacter() throws {
    #expect(try parseJSON(#""😀""#) == .text("\u{1F600}"))
    #expect(try parseJSON(#""\ud83d""#) == .text("\u{FFFD}"))
    #expect(try parseJSON(#""\ude00""#) == .text("\u{FFFD}"))
    #expect(try parseJSON(#""é""#) == .text("\u{E9}"))
}

@Test func aTextIsComparedByItsBytesAndNotByCanonicalEquivalence() throws {
    // "e" + U+0301 and U+00E9 are one string to Swift's `==` and two on the wire.
    #expect(JSONValue.text("e\u{301}") != JSONValue.text("\u{E9}"))
    let value = try parseJSON("{\"\u{E9}\":1}")
    #expect(value.member("e\u{301}") == nil)
    #expect(value.member("\u{E9}") == .number("1"))
}
