// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceLua

// Writes to a system variable (§scxml-5.10): "The Processor MUST cause any attempt to change the
// value of a system variable to fail and MUST place the error 'error.execution' on the internal
// event queue." §scxml-B-2-8 says what is read-only in the ECMAScript data model: the variable, the
// fields of `_event`, and each `_ioprocessors` entry with its `location`. A refused write is a chunk
// that fails, a thrown `LuaError.runtime` here, which the generated machine turns into
// `error.execution`.
//
// Which writes are refused is not written here. The expectations live in the table every SCE script
// engine reads, `tests/ecmascript/system_variable_writes.json`, with the session and the event every
// case runs against; this file holds this backend's Lua engine to it.

/// The table's `session`, as this binding takes one. An I/O processor the table names that this
/// binding cannot publish is an error, and not an entry left out quietly.
private func tableSession(_ table: [String: Any]) throws -> LuaSession {
    let session = try #require(table["session"] as? [String: Any])
    let processors = try #require(session["ioprocessors"] as? [String: String])
    let unknown = Set(processors.keys).subtracting(["scxml", "basichttp"])
    #expect(unknown.isEmpty, "the table's session names I/O processors this binding cannot publish: \(unknown.sorted())")
    let sessionID = try #require(session["sessionid"] as? String)
    let name = try #require(session["name"] as? String)
    let scxmlLocation = try #require(processors["scxml"])
    return LuaSession(
        sessionID: sessionID, name: name, scxmlLocation: scxmlLocation,
        basicHTTPLocation: processors["basichttp"])
}

/// The table's `event`, as this binding takes one. A field the table leaves out is blank.
private func tableEvent(_ table: [String: Any]) throws -> LuaEvent {
    let fields = try #require(table["event"] as? [String: Any])
    func field(_ key: String) -> String? { fields[key] as? String }
    let name = try #require(field("name"))
    return LuaEvent(
        name: name, type: field("type"), sendID: field("sendid"), origin: field("origin"),
        originType: field("origintype"), invokeID: field("invokeid"), data: field("data"))
}

/// A state with the library loaded and the table's system variables bound, as a machine's own is.
private func boundState(_ session: LuaSession, _ event: LuaEvent) throws -> LuaState {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.bindSession(session)
    try lua.bindCurrentEvent(event)
    return lua
}

@Test func everySystemVariableWriteIsRefusedOrNotTheWayTheSharedTableSays() throws {
    let table = try sharedTable("system_variable_writes.json")
    let rows = try #require(table["cases"] as? [[String: Any]])
    // A floor, not an equality: adding a case must not have to touch this number, but a table that
    // stopped being read must not pass either.
    #expect(rows.count >= 18, "the shared system-variable table produced only \(rows.count) case(s)")
    let session = try tableSession(table)
    let event = try tableEvent(table)

    var asked = 0
    var failures: [String] = []
    for (index, row) in rows.enumerated() {
        let write = row["write_lua"] as? String ?? ""
        let after = row["after_lua"] as? String ?? ""
        let clause = row["clause"] as? String ?? ""
        // A case missing a key is not a case that passes: reading it as "nothing to check" would
        // let a typo in the table retire a case silently.
        guard let refused = row["refused"] as? Bool, !write.isEmpty, !after.isEmpty else {
            failures.append("case \(index): names no write_lua, after_lua or refused")
            continue
        }
        do {
            // A state of its own per case, so a write that landed cannot answer for the next one.
            let lua = try boundState(session, event)
            asked += 1
            var failure: LuaError?
            do {
                try lua.execute(write, name: "=write")
            } catch let error as LuaError {
                failure = error
            }
            switch (refused, failure) {
            case (true, nil):
                failures.append("case \(index): `\(write)` ran; the table says it is refused (\(clause))")
            case (true, .runtime?), (false, nil):
                break
            case (true, let other?):
                // Refusal is a run-time failure. A write that does not parse fails too, and would
                // pass as refused while nothing was asked of the binding.
                failures.append("case \(index): `\(write)` failed as \(other), not as a refused write")
            case (false, let other?):
                failures.append("case \(index): `\(write)` failed (\(other)); the table says it is not refused")
            }
            // The value the write would have changed, read back: a binding cannot pass by reporting
            // a failure while the write landed.
            if try !lua.evaluateCondition(after, chunkName: "=after") {
                failures.append("case \(index): after `\(write)`, `\(after)` does not hold (\(clause))")
            }
        } catch {
            failures.append("case \(index): `\(write)`: \(error)")
        }
    }
    // A case that was never asked is not a case that passed.
    #expect(asked == rows.count, "asked \(asked) of the \(rows.count) cases the shared table holds")
    #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
}

// The name a `<data id>` or a parent's `<param>` gives a variable is the document's, and it reaches
// `setGlobal` and `defineGlobal` as a string. One that is a system variable's is refused there as
// well (§scxml-5.10), inside the shim's protected call, and the bound value is what it was.
@Test func aGlobalTheDocumentNamesIsRefusedWhenItIsASystemVariable() throws {
    let table = try sharedTable("system_variable_writes.json")
    let session = try tableSession(table)
    let event = try tableEvent(table)
    let lua = try boundState(session, event)
    for name in ["_event", "_sessionid", "_name", "_ioprocessors"] {
        #expect(throws: LuaError.self, "setGlobal(\(name))") { try lua.setGlobal(name, .string("x")) }
        #expect(throws: LuaError.self, "defineGlobal(\(name))") { try lua.defineGlobal(name, as: "'x'") }
    }
    #expect(try lua.evaluate("_event.name") == .string(event.name))
    #expect(try lua.global("_sessionid") == .string(session.sessionID))
    #expect(try lua.global("_name") == .string(session.name))
    #expect(try lua.evaluate("_ioprocessors.scxml.location") == .string(session.scxmlLocation))

    // The positive control: a name of the document's own is written both ways. Without it the
    // refusals above would be satisfied by a state that refused every global.
    try lua.setGlobal("plain", .string("x"))
    try lua.defineGlobal("defined", as: "1 + 1")
    #expect(try lua.global("plain") == .string("x"))
    #expect(try lua.global("defined") == .integer(2))
}

// The system variables are bound through `_scxml_bind_system`, which the library defines. A state
// without it cannot bind them, says why, and leaves none of the transient globals behind.
@Test func bindingASystemVariableBeforeTheLibraryIsLoadedFailsAndSaysWhy() throws {
    let lua = try LuaState()
    do {
        try lua.bindSession(LuaSession(sessionID: "s", name: "m", scxmlLocation: "#_scxml_s", basicHTTPLocation: "http://h/"))
        Issue.record("bindSession bound a session in a state with no library")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("loadEcmaScriptLibrary"), "\(message)")
    }
    do {
        try lua.bindCurrentEvent(LuaEvent(name: "e", type: "external", data: "x"))
        Issue.record("bindCurrentEvent bound an event in a state with no library")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("loadEcmaScriptLibrary"), "\(message)")
    }
    for name in [
        "_sce_session_id", "_sce_session_name", "_sce_http_location", "_sce_event_name",
        "_sce_event_type", "_sce_event_raw", "_sessionid", "_name", "_ioprocessors", "_event",
    ] {
        #expect(try lua.global(name) == .nilValue, "\(name)")
    }
}

// Rebinding is the processor's and not a write: the next event replaces `_event`, and the new one is
// as read-only as the first.
@Test func theNextEventIsBoundThroughTheSameRefusal() throws {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.bindCurrentEvent(LuaEvent(name: "first", type: "external"))
    try lua.bindCurrentEvent(LuaEvent(name: "second", type: "internal", data: #"{"n":1}"#))
    #expect(try lua.evaluate("_event.name") == .string("second"))
    #expect(throws: LuaError.self) { try lua.execute("_event.name = 'x'") }
    #expect(try lua.evaluate("_event.name") == .string("second"))
    try lua.execute("_event.data.n = 2")
    #expect(try lua.evaluate("_event.data.n") == .integer(2))
}
