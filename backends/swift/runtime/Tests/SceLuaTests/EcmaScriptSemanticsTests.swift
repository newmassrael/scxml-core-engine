// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceLua

// What this backend answers for `datamodel="ecmascript"`, measured against what ECMA-262 says.
//
// The expectations are not ours: `tests/ecmascript/ecma262_semantics.json` holds one claim per case
// with the clause that backs it, and every engine reads that one file. Swift never sees the
// ECMAScript, because the frontend lowered it to Lua at build time; the Lua it runs for each case is
// `tests/ecmascript/ecma262_emitted_lua.json`, and what this measures is the half Swift owns: the
// shared library the module embeds, the Lua 5.4 it links, and the conversion of what a chunk returns.
// Every case gets a state of its own, so a case that assumed an undeclared name sees the library and
// nothing an earlier case left behind.

/// The repository root, found from this file's own path: the tables every engine reads live under
/// `tests/` at the root, and the tests read them where they are and do not copy them.
private func repositoryRoot(file: String = #filePath) -> URL {
    // <root>/backends/swift/runtime/Tests/SceLuaTests/<this file>
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

private func cases(of table: String) throws -> [[String: Any]] {
    let url = repositoryRoot().appendingPathComponent("tests/ecmascript").appendingPathComponent(table)
    let object = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any]
    return try #require(object?["cases"] as? [[String: Any]])
}

/// Whether `actual` is the answer `expect` names. An engine may hold a whole number as an integer or
/// as a double, and ECMA-262 has one Number type, so both spellings answer a `number` case. A case
/// whose expectation names no answer is not a case that passes: reading it as "no answer" would let a
/// typo in a key retire a case silently.
private func answers(_ actual: LuaValue, _ expect: [String: Any]) throws -> Bool {
    if let want = expect["bool"] as? Bool {
        return actual == .boolean(want)
    }
    if let want = expect["number"] as? Double {
        switch actual {
        case .integer(let value): return abs(Double(value) - want) < 1e-9
        case .number(let value): return abs(value - want) < 1e-9
        default: return false
        }
    }
    if let want = expect["string"] as? String {
        return actual == .string(want)
    }
    if expect["empty"] != nil {
        return actual == .nilValue
    }
    throw LuaError.runtime("an expectation that names no answer: \(expect)")
}

@Test func everyCaseOfTheSharedTableIsAnsweredAsEcmaScriptAnswers() throws {
    let table = try cases(of: "ecma262_semantics.json")
    let emitted = try cases(of: "ecma262_emitted_lua.json")

    // A floor, not an equality: adding a case must not have to touch this number, but a table that
    // stopped being read must not pass either.
    #expect(table.count >= 55, "the shared table produced only \(table.count) case(s)")
    try #require(table.count == emitted.count, "the table holds \(table.count) case(s) and the emission \(emitted.count)")

    var failures: [String] = []
    for (index, expected) in table.enumerated() {
        let emission = emitted[index]
        let source = expected["source"] as? String ?? ""
        // The pairing is asserted, not assumed: an emission regenerated from a table that has since
        // gained a row would line case 40 up against case 41's answer and report a defect that is
        // really a stale file.
        guard source == emission["source"] as? String else {
            failures.append("case \(index) is `\(source)` in the table and `\(emission["source"] ?? "")` in the emission")
            continue
        }
        let setup = emission["setup"] as? String ?? ""
        let expression = emission["expression"] as? String ?? ""
        let clause = expected["clause"] as? String ?? ""
        do {
            let lua = try LuaState()
            try lua.loadEcmaScriptLibrary()
            if !setup.isEmpty {
                try lua.execute(setup, name: "=setup")
            }
            let actual = try lua.evaluate(expression, name: "=expression")
            if try !answers(actual, expected["expect"] as? [String: Any] ?? [:]) {
                failures.append("[\(source)] answered \(actual), ECMAScript says \(expected["expect"] ?? "?") (\(clause))\n  emitted: \(expression)")
            }
        } catch {
            failures.append("[\(source)] \(error)\n  emitted: \(expression)")
        }
    }
    #expect(failures.isEmpty, "\(failures.count) of \(table.count) expressions disagree with ECMA-262 on this backend:\n\(failures.joined(separator: "\n"))")
}

// The sweep above passes when every answer matches, so it is only worth anything if the comparison
// can say no: a wrong answer, a wrong kind of answer and an expectation that names no answer.
@Test func theComparisonRefusesAWrongAnswerAndAnExpectationThatNamesNone() throws {
    #expect(try answers(.integer(3), ["number": 3.0]))
    #expect(try answers(.number(0.5), ["number": 0.5]))
    #expect(try !answers(.integer(3), ["number": 4.0]))
    #expect(try !answers(.string("3"), ["number": 3.0]))
    #expect(try answers(.boolean(false), ["bool": false]))
    #expect(try !answers(.boolean(false), ["bool": true]))
    #expect(try !answers(.integer(0), ["bool": false]))
    #expect(try answers(.string("a"), ["string": "a"]))
    #expect(try !answers(.string("a"), ["string": "b"]))
    #expect(try answers(.nilValue, ["empty": true]))
    #expect(try !answers(.boolean(false), ["empty": true]))
    #expect(throws: LuaError.self) { try answers(.nilValue, [:]) }
}

@Test func theLibraryIsWhatDefinesTheVocabularyTheEmittedCodeCalls() throws {
    let bare = try LuaState()
    #expect(throws: LuaError.self) { try bare.evaluate("_scxml_truthy(0)") }

    let loaded = try LuaState()
    try loaded.loadEcmaScriptLibrary()
    #expect(try loaded.evaluate("_scxml_truthy(0)") == .boolean(false))
    #expect(try loaded.evaluate("_scxml_truthy('x')") == .boolean(true))
}

@Test func theLibraryLoadsTwiceWithoutHarm() throws {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.loadEcmaScriptLibrary()
    #expect(try lua.evaluate("_scxml_truthy(1)") == .boolean(true))
}
