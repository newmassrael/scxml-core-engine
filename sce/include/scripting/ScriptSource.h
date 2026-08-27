// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string>
#include <utility>

namespace SCE {

/**
 * @brief Which language a string handed to a script engine is written in.
 *
 * The spellings match the `script_engine_language` wire vocabulary
 * (`sce-build/src/manifest.rs`, `SCRIPT_ENGINE_LANGUAGES`) so the manifest a
 * host reads and the tag the engine is handed cannot drift into two different
 * names for one answer.
 */
enum class ScriptLanguage {
    /// The author's own text, as written in the SCXML document under
    /// `datamodel="ecmascript"`. An engine that does not evaluate ECMAScript
    /// must adapt it (`EcmaScriptToLuaTransformer`) or refuse.
    ECMAScript,
    /// Lua that `sce-build`'s ECMAScript frontend already produced, via the
    /// `to_lua_*` filters. Nothing further is to be rewritten.
    Lua,
};

/// The wire spelling of a language, for diagnostics and manifest comparison.
inline const char *scriptLanguageName(ScriptLanguage language) {
    return language == ScriptLanguage::Lua ? "lua" : "ecmascript";
}

/**
 * @brief A script or expression crossing into an engine, with its language.
 *
 * Two strings, not one. `text()` is what the engine evaluates; `source()` is
 * the author's ECMAScript, which is what a diagnostic has to name. They are
 * the same string only when the engine is handed the author's own text.
 *
 * The pairing is not a formatting nicety. `LuaEngine::evaluateExpressionInternal`
 * runs its undeclared-variable check on the *lowered* text and then builds
 * `ReferenceError: <expr> is not defined` from the *original* — and that
 * message travels out on `_event.data` of `error.execution`, so an entry point
 * that received only lowered Lua would name a language the author never wrote.
 * (The clause that check keeps is cited on that function, not here: this type
 * carries the two strings, it does not implement the semantics.)
 * See docs/SCE_LUA_TRANSLATION_SEAM.md, "So the seam cannot be a one-string
 * signature".
 *
 * There is deliberately no one-argument `lua()` form. A caller with no
 * authored ECMAScript to pass must pass the Lua twice, and thereby state that
 * its diagnostics will name Lua.
 */
class ScriptSource {
public:
    /// The author's text, to be adapted by the engine if it does not evaluate
    /// ECMAScript. This is what every call site that predates the seam means.
    static ScriptSource ecmascript(std::string source) {
        std::string text = source;
        return ScriptSource(ScriptLanguage::ECMAScript, std::move(text), std::move(source));
    }

    /// Lua the build-time frontend already produced, paired with the
    /// ECMAScript it was lowered from.
    static ScriptSource lua(std::string lowered, std::string source) {
        return ScriptSource(ScriptLanguage::Lua, std::move(lowered), std::move(source));
    }

    /// The language of `text()`.
    ScriptLanguage language() const {
        return language_;
    }

    /// What the engine evaluates.
    const std::string &text() const {
        return text_;
    }

    /// The author's ECMAScript, for every diagnostic and log line that names
    /// the expression back to whoever wrote it.
    const std::string &source() const {
        return source_;
    }

private:
    ScriptSource(ScriptLanguage language, std::string text, std::string source)
        : language_(language), text_(std::move(text)), source_(std::move(source)) {}

    ScriptLanguage language_;
    std::string text_;
    std::string source_;
};

}  // namespace SCE
