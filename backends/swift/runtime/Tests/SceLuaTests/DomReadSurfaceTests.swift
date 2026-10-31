// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceLua

// The DOM a Lua state holds for XML in the data model, measured against the table every engine
// reads: `tests/ecmascript/dom_read_surface.json` names the documents, the Lua each lowered
// expression becomes, and the answer DOM Level 1 Core gives, with the clause behind it. Every case
// gets a state of its own with its document parsed into `var1`, the name the table's expressions use.

@Test func everyCaseOfTheSharedDomTableIsAnsweredAsDomLevelOneAnswers() throws {
    let url = repositoryRoot().appendingPathComponent("tests/ecmascript/dom_read_surface.json")
    let object = try #require(
        try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
    let documents = try #require(object["documents"] as? [String: String])
    let table = try cases(of: "dom_read_surface.json")

    // A floor, not an equality: a table that stopped being read must not pass.
    #expect(table.count >= 39, "the shared table produced only \(table.count) case(s)")

    var failures: [String] = []
    for expected in table {
        let source = expected["source"] as? String ?? ""
        let lua = expected["lua"] as? String ?? ""
        let clause = expected["clause"] as? String ?? ""
        let name = expected["document"] as? String ?? ""
        guard let markup = documents[name] else {
            failures.append("[\(source)] names the document `\(name)`, which the table does not hold")
            continue
        }
        do {
            let state = try LuaState()
            try state.loadEcmaScriptLibrary()
            try state.setGlobal("markup", .string(markup))
            try state.execute("var1 = _scxml_parse_xml(markup)", name: "=document")
            let actual = try state.evaluate(lua, name: "=expression")
            if try !answers(actual, expected["expect"] as? [String: Any] ?? [:]) {
                failures.append("[\(source)] answered \(actual), DOM says \(expected["expect"] ?? "?") (\(clause))\n  lua: \(lua)")
            }
        } catch {
            failures.append("[\(source)] \(error)\n  lua: \(lua)")
        }
    }
    #expect(failures.isEmpty, "\(failures.count) of \(table.count) reads disagree with the DOM on this backend:\n\(failures.joined(separator: "\n"))")
}

// Markup that is not well formed is an error of the data model that read it, not a half tree.
@Test func markupThatIsNotWellFormedIsRefused() throws {
    let state = try LuaState()
    try state.loadEcmaScriptLibrary()
    for markup in ["<a><b></a>", "<a>", "<a b=1/>", "", "<!-- only -->", "<a></a><"] {
        try state.setGlobal("markup", .string(markup))
        #expect(throws: LuaError.self, "`\(markup)` is not a document") {
            try state.execute("_scxml_parse_xml(markup)", name: "=document")
        }
    }
}

// The entity and character references of the five predefined entities and of numeric forms.
@Test func referencesInTextAndAttributesAreDecoded() throws {
    let state = try LuaState()
    try state.loadEcmaScriptLibrary()
    try state.setGlobal("markup", .string("<a t=\"x&amp;y&#65;&#x42;\">1 &lt; 2 &apos;&quot;</a>"))
    try state.execute("var1 = _scxml_parse_xml(markup)", name: "=document")
    #expect(try state.evaluate("var1:getAttribute('t')") == .string("x&yAB"))
    #expect(try state.evaluate("var1.textContent") == .string("1 < 2 '\""))
}
