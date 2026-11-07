// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The four system variables a document reads and cannot change: `_event`, `_sessionid`, `_name` and
// `_ioprocessors`. §scxml-5.10: "The Processor MUST cause any attempt to change the value of a
// system variable to fail and MUST place the error 'error.execution' on the internal event queue."
// A Lua global is an ordinary table slot that takes any write, so the rule is not this module's to
// spell: `ecma_semantics.lua`, which every Lua engine loads, defines `_scxml_bind_system`, keeps the
// four outside `_G` behind read-only views (§scxml-B-2-8: `_event` and its fields, `_ioprocessors`,
// each entry and its `location`) and refuses an ordinary global write of any of the four names.
// This module binds them through that function and no other way: a chunk that set one as a plain
// global fails, which is how a binding that went around the function is found.
//
// A refused write is a chunk that fails, a thrown `LuaError.runtime` here, which the generated
// machine turns into `error.execution`. `tests/ecmascript/system_variable_writes.json` says which
// writes are refused, for every engine.

extension LuaState {
    /// The opening of every chunk that binds a system variable: it gives the chunk the local `bind`,
    /// which is `_scxml_bind_system`. A state the library was never loaded into has none, and the
    /// chunk fails saying so rather than as a call of nil part way through a binding.
    static let systemVariableBinder = """
        local bind = _scxml_bind_system
        if type(bind) ~= 'function' then
            error('a system variable is bound through _scxml_bind_system, which loadEcmaScriptLibrary() defines and this state has not loaded', 0)
        end
        """
}
