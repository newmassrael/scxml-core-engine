// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceLua

// The system variables and `In()` a document reads about its own session (§scxml-5.10, §scxml-C-1).

private let scxmlProcessor = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
private let httpProcessor = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

private func session(http: String? = nil) -> LuaSession {
    LuaSession(sessionID: "machine.7", name: "machine", scxmlLocation: "#_scxml_machine.7", basicHTTPLocation: http)
}

private func boundState(_ session: LuaSession) throws -> LuaState {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    try lua.bindSession(session)
    return lua
}

@Test func theSessionVariablesAreWhatTheSessionSaid() throws {
    let lua = try boundState(session())
    #expect(try lua.evaluate("_sessionid") == .string("machine.7"))
    #expect(try lua.evaluate("_name") == .string("machine"))
    #expect(try lua.evaluate("_scxml_location") == .string("#_scxml_machine.7"))
}

// The values cross as values. A name with a quote of either kind, or a bracket, is one string and
// not Lua source that stops parsing.
@Test func aNameWithQuotesAndBracketsIsStillOneString() throws {
    let awkward = #"it's "x" ]] \ end"#
    let lua = try boundState(LuaSession(sessionID: "s", name: awkward, scxmlLocation: "#_scxml_s"))
    #expect(try lua.evaluate("_name") == .string(awkward))
}

@Test func theScxmlProcessorIsFiledUnderBothItsNames() throws {
    let lua = try boundState(session())
    #expect(try lua.evaluate("_ioprocessors['\(scxmlProcessor)'].location") == .string("#_scxml_machine.7"))
    #expect(try lua.evaluate("_ioprocessors.scxml.location") == .string("#_scxml_machine.7"))
}

@Test func aSessionWithNoListenerPublishesNoBasicHttpEntry() throws {
    let lua = try boundState(session())
    #expect(try lua.evaluate("_ioprocessors.basichttp") == .nilValue)
    #expect(try lua.evaluate("_ioprocessors['\(httpProcessor)']") == .nilValue)

    let empty = try boundState(session(http: ""))
    #expect(try empty.evaluate("_ioprocessors.basichttp") == .nilValue)
}

@Test func aDeclaredListenerIsPublishedUnderBothNames() throws {
    let lua = try boundState(session(http: "http://127.0.0.1:8080/scxml"))
    #expect(try lua.evaluate("_ioprocessors.basichttp.location") == .string("http://127.0.0.1:8080/scxml"))
    #expect(try lua.evaluate("_ioprocessors['\(httpProcessor)'].location") == .string("http://127.0.0.1:8080/scxml"))
}

// Every entry is a table of its own: a document that writes through one name must not change what
// the other answers.
@Test func theEntriesOfOneProcessorDoNotShareATable() throws {
    let lua = try boundState(session(http: "http://h/"))
    try lua.execute("_ioprocessors.scxml.location = 'changed'")
    #expect(try lua.evaluate("_ioprocessors['\(scxmlProcessor)'].location") == .string("#_scxml_machine.7"))
    try lua.execute("_ioprocessors.basichttp.location = 'changed'")
    #expect(try lua.evaluate("_ioprocessors['\(httpProcessor)'].location") == .string("http://h/"))
}

@Test func bindingAgainReplacesTheTableAndTheTransientGlobalIsGone() throws {
    let lua = try boundState(session(http: "http://h/"))
    try lua.bindSession(session())
    #expect(try lua.evaluate("_ioprocessors.basichttp") == .nilValue)
    #expect(try lua.global("_sce_http_location") == .nilValue)
}

@Test func inAsksTheHostAndSeesItsConfigurationChange() throws {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    var active: Set<String> = ["a", "b.c"]
    try lua.setStateQuery { active.contains($0) }

    #expect(try lua.evaluate("In('a')") == .boolean(true))
    #expect(try lua.evaluate("In('b.c')") == .boolean(true))
    #expect(try lua.evaluate("In('x')") == .boolean(false))
    #expect(try lua.evaluate("In('a') and not In('x')") == .boolean(true))

    active = ["x"]
    #expect(try lua.evaluate("In('a')") == .boolean(false))
    #expect(try lua.evaluate("In('x')") == .boolean(true))
}

@Test func inIsAskedTheStateIdExactlyAsWritten() throws {
    let lua = try LuaState()
    var asked: [String] = []
    try lua.setStateQuery {
        asked.append($0)
        return false
    }
    _ = try lua.evaluate("In('한글') or In('a b') or In(42)")
    #expect(asked == ["한글", "a b", "42"])
}

// A state id is a string. An argument that is not one raises in Lua before the host is asked, and
// a raise is a thrown error here and not a jump through the host's frame.
@Test func inWithAnArgumentThatIsNotAStringRaisesWithoutAskingTheHost() throws {
    let lua = try LuaState()
    var asked = 0
    try lua.setStateQuery { _ in
        asked += 1
        return true
    }
    #expect(throws: LuaError.self) { try lua.evaluate("In({})") }
    #expect(throws: LuaError.self) { try lua.evaluate("In()") }
    #expect(asked == 0)
    #expect(try lua.evaluate("In('a')") == .boolean(true))
}

@Test func removingTheQueryRemovesIn() throws {
    let lua = try LuaState()
    try lua.setStateQuery { _ in true }
    #expect(try lua.evaluate("In('a')") == .boolean(true))
    try lua.setStateQuery(nil)
    #expect(try lua.evaluate("In") == .nilValue)
    #expect(throws: LuaError.self) { try lua.evaluate("In('a')") }
}

@Test func replacingTheQueryAnswersFromTheNewOne() throws {
    let lua = try LuaState()
    try lua.setStateQuery { _ in true }
    try lua.setStateQuery { _ in false }
    #expect(try lua.evaluate("In('a')") == .boolean(false))
}

@Test func aSessionBoundBeforeAndAfterAnEventStillAnswersBothThings() throws {
    let lua = try boundState(session())
    try lua.setStateQuery { $0 == "running" }
    try lua.bindCurrentEvent(LuaEvent(name: "go", type: "external", data: #"{"n":2}"#))
    #expect(try lua.evaluate("In('running') and _event.data.n == 2 and _name == 'machine'") == .boolean(true))
}
