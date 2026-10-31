// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceLua

// The two operations a lowered machine uses beyond `evaluate`: a `<data id expr>` initialiser, which
// gives a global the value of an expression whatever its kind, and a transition `cond`, which is a
// boolean or a defect.

@Test func aGlobalTakesTheValueOfAnExpression() throws {
    let lua = try LuaState()
    try lua.execute("x = 2")
    try lua.defineGlobal("y", as: "x * 21")
    #expect(try lua.global("y") == .integer(42))
}

// A `<data id>` is an XML name. `screen-rules` is a subtraction to Lua, so a declaration that wrote
// `screen-rules = (4)` as source was never initialised.
@Test func theNameOfAGlobalIsAStringAndNeverSource() throws {
    let lua = try LuaState()
    try lua.defineGlobal("screen-rules", as: "(4)")
    #expect(try lua.global("screen-rules") == .integer(4))
    #expect(try lua.evaluate("_G['screen-rules']") == .integer(4))
    try lua.defineGlobal("a.b", as: "'dotted'")
    #expect(try lua.evaluate("_G['a.b']") == .string("dotted"))
}

// `LuaValue` cannot carry a table out, so a `<data>` whose initialiser is an array or an object has
// to be stored where it is made.
@Test func aTableIsHeldByTheSessionThoughNoSwiftValueCanCarryIt() throws {
    let lua = try LuaState()
    try lua.defineGlobal("list", as: "{10, 20, 30}")
    #expect(try lua.evaluate("#list") == .integer(3))
    #expect(try lua.evaluate("list[2]") == .integer(20))
    #expect(try lua.global("list") == .other(typeName: "table"))
}

@Test func aFailedDefinitionNamesItsKindAndLeavesTheGlobalAlone() throws {
    let lua = try LuaState()
    try lua.setGlobal("kept", .integer(1))
    #expect(throws: LuaError.self) { try lua.defineGlobal("kept", as: "nil + 1") }
    #expect(try lua.global("kept") == .integer(1))
    do {
        try lua.defineGlobal("kept", as: "1 +")
        Issue.record("a syntax error was not thrown")
    } catch LuaError.syntax {
    }
    #expect(try lua.global("kept") == .integer(1))
}

@Test func aRaisingMetatableOnTheGlobalsIsAThrownErrorWhenADefinitionWrites() throws {
    let lua = try LuaState()
    try lua.execute("setmetatable(_G, {__newindex = function() error('frozen') end})")
    do {
        try lua.defineGlobal("x", as: "1")
        Issue.record("a definition into a frozen global table did not throw")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("frozen"), "\(message)")
    }
}

@Test func aConditionIsABooleanOrAThrownError() throws {
    let lua = try LuaState()
    try lua.loadEcmaScriptLibrary()
    #expect(try lua.evaluateCondition("1 < 2"))
    #expect(try !lua.evaluateCondition("1 > 2"))
    #expect(try lua.evaluateCondition("_scxml_truthy('a')"))
    #expect(try !lua.evaluateCondition("_scxml_truthy(0)"))
    // A lowered condition answers a boolean. Anything else is a defect of the lowering, which would
    // take a transition, or not, on a guess nobody could see.
    #expect(throws: LuaError.self) { try lua.evaluateCondition("1") }
    #expect(throws: LuaError.self) { try lua.evaluateCondition("nil") }
    #expect(throws: LuaError.self) { try lua.evaluateCondition("'true'") }
    #expect(throws: LuaError.self) { try lua.evaluateCondition("1 <") }
}
