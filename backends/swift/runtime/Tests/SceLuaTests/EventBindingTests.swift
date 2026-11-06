// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceLua

// `_event` as a document reads it: which reading each payload gets (§scxml-B-2-8-1), what an event
// that carries no data leaves, and that a payload is only ever read and never run.

private func boundState(_ event: LuaEvent) throws -> LuaState {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.bindCurrentEvent(event)
    return lua
}

@Test func everyPayloadTheClauseNamesIsReadTheWayTheSharedTableSays() throws {
    let table = try cases(of: "event_data_readings.json")
    // A floor, not an equality: adding a case must not have to touch this number, but a table that
    // stopped being read must not pass either.
    #expect(table.count >= 8, "the shared reading table produced only \(table.count) case(s)")

    var failures: [String] = []
    for expected in table {
        let payload = expected["payload"] as? String ?? ""
        let expression = expected["lua"] as? String ?? ""
        do {
            let lua = try boundState(LuaEvent(name: "brief", type: "external", data: payload))
            let actual = try lua.evaluate(expression, name: "=expression")
            if try !answers(actual, expected["expect"] as? [String: Any] ?? [:]) {
                failures.append("payload \(payload.debugDescription): \(expression) answered \(actual), the table says \(expected["expect"] ?? "?")")
            }
        } catch {
            failures.append("payload \(payload.debugDescription): \(error)")
        }
    }
    #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
}

@Test func anEventWithNoDataLeavesTheFieldUndefinedAndTheReadOfAFieldOffItFails() throws {
    let lua = try boundState(LuaEvent(name: "judge", type: "external"))
    #expect(try lua.evaluate("_event.data") == .nilValue)
    // A cond doing this is a failed cond, which raises error.execution; a binding that answered
    // nothing instead would make a verdict that never came look like one that said no.
    #expect(throws: LuaError.self) { try lua.evaluate("_event.data.done") }

    // The positive control: the same read once a payload has arrived. Without it the failure above
    // would be satisfied by a binding that refused every field read.
    try lua.bindCurrentEvent(LuaEvent(name: "judge", type: "external", data: #"{"done":true}"#))
    #expect(try lua.evaluate("_event.data.done") == .boolean(true))
}

@Test func theNextEventReplacesThePreviousOneWhole() throws {
    let lua = try boundState(
        LuaEvent(
            name: "first", type: "external", sendID: "s1", origin: "#_scxml_a", originType: "scxml",
            data: #"{"a":1}"#))
    #expect(try answers(lua.evaluate("_event.data.a"), ["number": 1.0]))
    try lua.bindCurrentEvent(LuaEvent(name: "second", type: "internal"))
    #expect(try lua.evaluate("_event.name") == .string("second"))
    #expect(try lua.evaluate("_event.data") == .nilValue)
    #expect(try lua.evaluate("_event.sendid") == .nilValue)
    #expect(try lua.evaluate("_event.raw") == .nilValue)
    #expect(try lua.evaluate("_event.origin") == .nilValue)
    #expect(try lua.evaluate("_event.origintype") == .nilValue)
}

// What a blank field reads as (§scxml-B-2-8: ECMAScript undefined, which Lua holds as nil), by the
// table every SCE script engine reads. A member a case leaves out of its `event` is a field the
// Processor left blank, handed to the binding as the empty text, as the engine interface carries it.
@Test func everyBlankFieldReadsTheWayTheSharedTableSays() throws {
    let table = try cases(of: "event_blank_fields.json")
    // A floor, not an equality: adding a case must not have to touch this number, but a table that
    // stopped being read must not pass either.
    #expect(table.count >= 11, "the shared blank-field table produced only \(table.count) case(s)")

    var asked = 0
    var failures: [String] = []
    for (index, expected) in table.enumerated() {
        let event = expected["event"] as? [String: Any] ?? [:]
        func field(_ key: String) -> String { event[key] as? String ?? "" }
        let expression = expected["lua"] as? String ?? ""
        do {
            let lua = try boundState(
                LuaEvent(
                    name: field("name"), type: field("type"), sendID: field("sendid"),
                    origin: field("origin"), originType: field("origintype"),
                    invokeID: field("invokeid")))
            asked += 1
            let actual = try lua.evaluate(expression, name: "=expression")
            if try !answers(actual, expected["expect"] as? [String: Any] ?? [:]) {
                failures.append("case \(index): \(expression) answered \(actual), the table says \(expected["expect"] ?? "?")")
            }
        } catch {
            failures.append("case \(index): \(expression): \(error)")
        }
    }
    // A case that was never asked is not a case that passed.
    #expect(asked == table.count, "asked \(asked) of the \(table.count) cases the shared table holds")
    #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
}

// §scxml-5.10.1 (docs/adr/0090, section 1): an event no Event I/O Processor delivered has its
// origin and origintype left blank, and §scxml-B-2-8 makes blank undefined: nil here, which a guard
// `_event.origin === undefined` finds and `_event.origin === ''` does not.
@Test func anEventWithNoOriginLeavesBothOriginFieldsAbsent() throws {
    let lua = try boundState(LuaEvent(name: "posted", type: "external"))
    #expect(try lua.evaluate("_event.origin") == .nilValue)
    #expect(try lua.evaluate("_event.origintype") == .nilValue)
    #expect(try lua.evaluateCondition("_event.origin == nil and _event.origintype == nil"))

    // An origin spelled out as empty reads as one left out.
    try lua.bindCurrentEvent(LuaEvent(name: "reply", type: "external", origin: "", originType: ""))
    #expect(try lua.evaluate("_event.origin") == .nilValue)
    #expect(try lua.evaluate("_event.origintype") == .nilValue)
}

// `type`, `sendid`, `origin`, `origintype` and `invokeid` are absent when empty
// (`anEventWithNoOriginLeavesBothOriginFieldsAbsent`).
@Test func theMetadataIsWhatTheEventCarriedAndAnEmptyFieldIsAbsent() throws {
    let lua = try boundState(
        LuaEvent(
            name: "reply", type: "external", sendID: "s1", origin: "#_scxml_a", originType: "scxml",
            invokeID: "child.1", data: "plain"))
    #expect(try lua.evaluate("_event.name") == .string("reply"))
    #expect(try lua.evaluate("_event.type") == .string("external"))
    #expect(try lua.evaluate("_event.sendid") == .string("s1"))
    #expect(try lua.evaluate("_event.origin") == .string("#_scxml_a"))
    #expect(try lua.evaluate("_event.origintype") == .string("scxml"))
    #expect(try lua.evaluate("_event.invokeid") == .string("child.1"))
    #expect(try lua.evaluate("_event.raw") == .string("plain"))

    try lua.bindCurrentEvent(LuaEvent(name: "bare", type: "", sendID: "", invokeID: ""))
    #expect(try lua.evaluate("_event.type") == .nilValue)
    #expect(try lua.evaluate("_event.sendid") == .nilValue)
    #expect(try lua.evaluate("_event.invokeid") == .nilValue)
}

@Test func theTransientGlobalsTheBindingUsesAreGoneAfterwards() throws {
    let lua = try boundState(LuaEvent(name: "e", type: "external", sendID: "s", data: "x"))
    for name in [
        "_sce_event_name", "_sce_event_type", "_sce_event_sendid", "_sce_event_origin",
        "_sce_event_origintype", "_sce_event_invokeid", "_sce_event_raw",
    ] {
        #expect(try lua.global(name) == .nilValue, "\(name)")
    }
}

// The payload is host input. As Lua source it was executed, so an event's data could run arbitrary
// code in the session, which is why it crosses as a value and is only ever parsed.
@Test func aPayloadThatIsACallNeverRuns() throws {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.execute("breached = false")
    let call = "(function() breached = true return 'x' end)()"
    try lua.bindCurrentEvent(LuaEvent(name: "brief", type: "external", data: call))
    #expect(try lua.global("breached") == .boolean(false))
    #expect(try lua.evaluate("_event.data") == .string(call))
}

@Test func aPayloadThatIsAnExpressionIsTheTextAndNotItsValue() throws {
    let lua = try boundState(LuaEvent(name: "brief", type: "external", data: "2 + 3"))
    #expect(try lua.evaluate("_event.data") == .string("2 + 3"))
}

@Test func theReadingTellsAStructuredPayloadFromProseFromARefusedOne() throws {
    func reading(_ data: String?) throws -> PayloadReading {
        let lua = try LuaState()
        try lua.loadEcmaScriptLibrary()
        return try lua.bindCurrentEvent(LuaEvent(name: "e", type: "external", data: data))
    }
    #expect(try reading(nil) == .absent)
    #expect(try reading("") == .absent)
    #expect(try reading("  \n\t ") == .absent)
    #expect(try reading(#"{"a":1}"#) == .structured)
    #expect(try reading("[1,2,3]") == .structured)
    #expect(try reading("hold   the line") == .text)
    #expect(try reading("2 + 3") == .text)
    #expect(try reading("<") == .text)
    #expect(try reading("<assign> to detail failed") == .text)
    #expect(try reading("<a/>") == .dom)
    #expect(try reading("  <a><b/></a>") == .dom)
    #expect(try reading("<a><b></a>") == .text)
    #expect(try reading("{not json") == .undecodable)
    #expect(try reading("[1,") == .undecodable)
}

@Test func aRefusedPayloadStillReadsAsTheNormalizedString() throws {
    let lua = try boundState(LuaEvent(name: "e", type: "external", data: "{ not   json"))
    #expect(try lua.evaluate("_event.data") == .string("{ not json"))
    #expect(try lua.evaluate("_event.raw") == .string("{ not   json"))
}
