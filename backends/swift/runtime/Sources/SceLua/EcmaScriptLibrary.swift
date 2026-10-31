// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The ECMAScript half of a Lua state: what the build-time lowering calls by name and Lua does not
// define. `sce/include/scripting/ecma_semantics.lua` carries the operators Lua spells differently
// (`+` is addition or concatenation by its operands, `==` coerces, the bitwise family acts on
// ToInt32) and the vocabulary the emitted code calls (`_scxml_truthy`, `_typeof`, `_indexOf`,
// `parseInt`); `json_builtins.lua` carries `JSON`. The Rust, C++, Go, Kotlin, Python and C11
// engines load the same two sources, so one document answers one way on every backend.

extension LuaState {
    /// Loads the ECMAScript operators and library into this state, in the order every engine loads
    /// them: the semantics first, then `JSON`. A source that fails to load throws; a state without
    /// them would answer wrongly at the first `+` of a document, which is why no failure is skipped.
    public func loadEcmaScriptLibrary() throws {
        try execute(
            String(decoding: PackageResources.ecma_semantics_lua, as: UTF8.self),
            name: "=ecma_semantics")
        try execute(
            String(decoding: PackageResources.json_builtins_lua, as: UTF8.self),
            name: "=json_builtins")
    }
}
