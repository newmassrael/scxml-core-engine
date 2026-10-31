// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import CLua

// The Lua 5.4 engine of a document whose expressions the generator lowers to Lua at build time
// (`datamodel="ecmascript"`): the optional tier every other Lua backend also keeps apart from its
// statechart runtime. A machine that needs no engine does not link it.

/// A value a Lua call returns or a global holds. A Lua string is a run of bytes; one that is not
/// UTF-8 text reads back with each bad sequence replaced.
public enum LuaValue: Equatable, Sendable {
    case nilValue
    case boolean(Bool)
    case integer(Int64)
    case number(Double)
    case string(String)
    /// A table, function, userdata or thread, named by its Lua type.
    case other(typeName: String)
}

/// How a call into Lua failed. A syntax error is a defect of the lowering, a run-time error is the
/// document's (it raises `error.execution`), and a memory error is neither.
public enum LuaError: Error, Equatable, Sendable {
    case syntax(String)
    case runtime(String)
    case memory
}

/// One Lua state with the standard libraries open. Every call runs under a protected call inside
/// the `CLua` shim, so a Lua error is a thrown `LuaError` and never a jump across a Swift frame.
///
/// A state is not thread safe and is not `Sendable`: one thread drives it, as it does the engine
/// that owns it.
public final class LuaState {
    // Module-visible, not public: the extensions of this module that bind a session or a callback
    // reach the shim through them.
    let handle: OpaquePointer
    /// What the shim's `In` calls back into, kept alive for as long as the shim may call it.
    var stateQuery: AnyObject?

    public init() throws {
        guard let handle = sce_lua_open() else {
            throw LuaError.memory
        }
        self.handle = handle
    }

    deinit {
        sce_lua_close(handle)
    }

    /// Runs `chunk`, which is text and not a precompiled chunk. `name` is what an error message
    /// quotes, in Lua's convention (`=guard` quotes `guard`).
    public func execute(_ chunk: String, name: String = "=chunk") throws {
        let status = withBytes(of: chunk) { bytes, length in
            name.withCString { sce_lua_execute(handle, bytes, length, $0) }
        }
        try check(status)
    }

    /// The value of `expression`, which is evaluated as `return (<expression>)`.
    public func evaluate(_ expression: String, name: String = "=expression") throws -> LuaValue {
        var raw = sce_lua_value()
        let status = withBytes(of: expression) { bytes, length in
            name.withCString { sce_lua_evaluate(handle, bytes, length, $0, &raw) }
        }
        try check(status)
        return Self.value(of: raw)
    }

    /// Gives the global `name` the value of `expression`, which is evaluated as
    /// `return (<expression>)`, whatever kind of value that is: a table or a function can be held by
    /// the session though `LuaValue` cannot carry it out. The name is a string and not source text, so
    /// a `<data id>` that is an XML name and not a Lua identifier (`screen-rules`) is still one global.
    public func defineGlobal(_ name: String, as expression: String, chunkName: String = "=expression") throws {
        let status = withBytes(of: expression) { bytes, length in
            name.withCString { global in
                chunkName.withCString { chunk in
                    sce_lua_define_global(handle, global, bytes, length, chunk)
                }
            }
        }
        try check(status)
    }

    /// Whether the condition `expression` holds. A lowered condition answers a boolean, so any other
    /// answer is a defect of the lowering and not a reading: it throws, where a guess of true or
    /// false would take a transition, or not, for no reason a document could see.
    public func evaluateCondition(_ expression: String, chunkName: String = "=condition") throws -> Bool {
        switch try evaluate(expression, name: chunkName) {
        case .boolean(let holds):
            return holds
        case let other:
            throw LuaError.runtime("the condition did not evaluate to a boolean: \(other)")
        }
    }

    /// Gives the global `name` the value `value`.
    public func setGlobal(_ name: String, _ value: LuaValue) throws {
        var raw = sce_lua_value()
        let status: sce_lua_status
        switch value {
        case .nilValue:
            raw.kind = SCE_LUA_NIL
            status = store(name, raw)
        case .boolean(let flag):
            raw.kind = SCE_LUA_BOOLEAN
            raw.integer = flag ? 1 : 0
            status = store(name, raw)
        case .integer(let number):
            raw.kind = SCE_LUA_INTEGER
            raw.integer = number
            status = store(name, raw)
        case .number(let number):
            raw.kind = SCE_LUA_NUMBER
            raw.number = number
            status = store(name, raw)
        case .string(let text):
            raw.kind = SCE_LUA_STRING
            status = withBytes(of: text) { bytes, length in
                raw.bytes = bytes
                raw.length = length
                return store(name, raw)
            }
        case .other(let typeName):
            throw LuaError.runtime("a value of the Lua type \(typeName) cannot be given to a global")
        }
        try check(status)
    }

    /// The value of the global `name`; one that was never set is `.nilValue`.
    public func global(_ name: String) throws -> LuaValue {
        var raw = sce_lua_value()
        let status = name.withCString { sce_lua_get_global(handle, $0, &raw) }
        try check(status)
        return Self.value(of: raw)
    }

    private func store(_ name: String, _ value: sce_lua_value) -> sce_lua_status {
        var value = value
        return name.withCString { sce_lua_set_global(handle, $0, &value) }
    }

    /// Hands `text` to `body` as bytes and a length, so a string with a NUL in it keeps its tail.
    private func withBytes<R>(of text: String, _ body: (UnsafePointer<CChar>, Int) -> R) -> R {
        text.utf8CString.withUnsafeBufferPointer { buffer in
            body(buffer.baseAddress!, buffer.count - 1)
        }
    }

    func check(_ status: sce_lua_status) throws {
        if status == SCE_LUA_OK {
            return
        }
        var length = 0
        let message = sce_lua_message(handle, &length).map { pointer in
            String(decoding: UnsafeRawBufferPointer(start: pointer, count: length), as: UTF8.self)
        }
        switch status {
        case SCE_LUA_SYNTAX:
            throw LuaError.syntax(message ?? "")
        case SCE_LUA_MEMORY:
            throw LuaError.memory
        default:
            throw LuaError.runtime(message ?? "")
        }
    }

    private static func value(of raw: sce_lua_value) -> LuaValue {
        let text = raw.bytes.map { pointer in
            String(decoding: UnsafeRawBufferPointer(start: pointer, count: raw.length), as: UTF8.self)
        }
        switch raw.kind {
        case SCE_LUA_BOOLEAN:
            return .boolean(raw.integer != 0)
        case SCE_LUA_INTEGER:
            return .integer(raw.integer)
        case SCE_LUA_NUMBER:
            return .number(raw.number)
        case SCE_LUA_STRING:
            return .string(text ?? "")
        case SCE_LUA_OTHER:
            return .other(typeName: text ?? "")
        default:
            return .nilValue
        }
    }
}
