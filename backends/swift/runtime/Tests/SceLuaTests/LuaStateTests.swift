// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceLua

// The Lua 5.4 state a lowered `datamodel="ecmascript"` document runs on: what comes back across the
// C boundary, and that every way a Lua call fails is a thrown error and never a jump through Swift.

@Test func aNewStateIsLua54() throws {
    let lua = try LuaState()
    #expect(try lua.evaluate("_VERSION") == .string("Lua 5.4"))
}

@Test func anIntegerStaysAnIntegerAndADivisionIsANumber() throws {
    let lua = try LuaState()
    #expect(try lua.evaluate("1 + 2") == .integer(3))
    #expect(try lua.evaluate("7 // 2") == .integer(3))
    #expect(try lua.evaluate("9223372036854775807") == .integer(Int64.max))
    #expect(try lua.evaluate("1 / 2") == .number(0.5))
    #expect(try lua.evaluate("2 ^ 2") == .number(4.0))
    #expect(try lua.evaluate("1 / 0") == .number(.infinity))
}

@Test func booleanNilStringAndTheTypeOfAnythingElse() throws {
    let lua = try LuaState()
    #expect(try lua.evaluate("1 < 2") == .boolean(true))
    #expect(try lua.evaluate("1 > 2") == .boolean(false))
    #expect(try lua.evaluate("nil") == .nilValue)
    #expect(try lua.evaluate("'a' .. 'b'") == .string("ab"))
    #expect(try lua.evaluate("{}") == .other(typeName: "table"))
    #expect(try lua.evaluate("print") == .other(typeName: "function"))
}

@Test func anExpressionEndingInACommentStillCloses() throws {
    let lua = try LuaState()
    #expect(try lua.evaluate("1 -- the rest of the line") == .integer(1))
}

@Test func aGlobalKeepsEveryKindOfValueItWasGiven() throws {
    let lua = try LuaState()
    let values: [LuaValue] = [
        .nilValue, .boolean(true), .boolean(false), .integer(0), .integer(Int64.min),
        .integer(Int64.max), .number(1.5), .number(-0.0), .string(""), .string("한글 text"),
    ]
    for (index, value) in values.enumerated() {
        let name = "g\(index)"
        try lua.setGlobal(name, value)
        #expect(try lua.global(name) == value, "\(value)")
        #expect(try lua.evaluate(name) == value, "\(value)")
    }
}

@Test func aStringWithANulKeepsItsTail() throws {
    let lua = try LuaState()
    try lua.setGlobal("s", .string("a\u{0}b"))
    #expect(try lua.evaluate("#s") == .integer(3))
    #expect(try lua.global("s") == .string("a\u{0}b"))
    #expect(try lua.evaluate("'x\\0y'") == .string("x\u{0}y"))
}

@Test func aGlobalThatWasNeverSetIsNil() throws {
    let lua = try LuaState()
    #expect(try lua.global("never_set") == .nilValue)
}

@Test func aChunkLeavesItsGlobalsForTheNextCall() throws {
    let lua = try LuaState()
    try lua.execute("count = 40; function twice(n) return n * 2 end")
    #expect(try lua.evaluate("twice(count) + 2") == .integer(82))
    #expect(try lua.global("count") == .integer(40))
}

@Test func aSyntaxErrorIsNamedAndIsNotARuntimeError() throws {
    let lua = try LuaState()
    do {
        try lua.execute("x = = 1", name: "=guard")
        Issue.record("a syntax error was not thrown")
    } catch LuaError.syntax(let message) {
        #expect(message.hasPrefix("guard:1:"), "\(message)")
    }
}

@Test func aRuntimeErrorCarriesItsMessageAndAStringErrorObjectIsTheMessage() throws {
    let lua = try LuaState()
    do {
        try lua.execute("error('boom')", name: "=script")
        Issue.record("a runtime error was not thrown")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("boom"), "\(message)")
    }
    do {
        try lua.execute("error({})")
        Issue.record("a runtime error was not thrown")
    } catch LuaError.runtime(let message) {
        #expect(message == "the error object is not a string")
    }
}

@Test func aFailedCallDoesNotSpoilTheNextOne() throws {
    let lua = try LuaState()
    #expect(throws: LuaError.self) { try lua.evaluate("nil + 1") }
    #expect(try lua.evaluate("1 + 1") == .integer(2))
    #expect(throws: LuaError.self) { try lua.evaluate("1 +") }
    #expect(try lua.evaluate("'ok'") == .string("ok"))
}

@Test func aPrecompiledChunkIsNeverLoaded() throws {
    let lua = try LuaState()
    do {
        try lua.execute("\u{1B}Lua")
        Issue.record("a binary chunk was loaded")
    } catch LuaError.syntax(let message) {
        #expect(message.contains("binary chunk"), "\(message)")
    }
}

@Test func aValueOfAnotherKindCannotBeGivenToAGlobal() throws {
    let lua = try LuaState()
    #expect(throws: LuaError.self) { try lua.setGlobal("t", .other(typeName: "table")) }
}

// A document can give `_G` a metatable whose handlers raise. Reading or writing a global then
// raises inside the call, which is why the shim runs both under a protected call: an unprotected
// one would end the process.
@Test func aRaisingMetatableOnTheGlobalsIsAThrownErrorNotACrash() throws {
    let lua = try LuaState()
    try lua.execute(
        """
        setmetatable(_G, {
            __index = function(_, key) error('no ' .. key) end,
            __newindex = function() error('frozen') end,
        })
        """)
    do {
        _ = try lua.global("missing")
        Issue.record("reading a missing global did not throw")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("no missing"), "\(message)")
    }
    do {
        try lua.setGlobal("x", .integer(1))
        Issue.record("writing a global did not throw")
    } catch LuaError.runtime(let message) {
        #expect(message.contains("frozen"), "\(message)")
    }
}

@Test func twoStatesDoNotShareGlobals() throws {
    let first = try LuaState()
    let second = try LuaState()
    try first.setGlobal("shared", .integer(1))
    #expect(try second.global("shared") == .nilValue)
}
