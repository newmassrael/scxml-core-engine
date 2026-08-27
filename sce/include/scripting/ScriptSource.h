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
    /**
     * @brief The author's own ECMAScript — and the implicit reading of a bare
     *        string anywhere a ScriptSource is expected.
     *
     * Implicit on purpose, the way `std::filesystem::path` takes a string.
     * Every call site that predates the seam hands over the author's text, so
     * that is what a bare string must continue to mean; making it explicit
     * would have churned hundreds of sites to say what they already said.
     *
     * The failure mode of the implicit reading is the safe one. A site that
     * SHOULD pass lowered Lua and forgets gets the author's text rewritten by
     * the engine — which is exactly today's behaviour, so a missed site stays
     * *diverging* rather than becoming newly wrong, and the ECMA-262 table is
     * what still reports it.
     */
    ScriptSource(std::string source)  // NOLINT(google-explicit-constructor)
        : language_(ScriptLanguage::ECMAScript), text_(source), source_(std::move(source)) {}

    /// A literal is a string too, and `const char *` → `std::string` →
    /// `ScriptSource` is one user conversion too many for the compiler to make
    /// on its own.
    ScriptSource(const char *source)  // NOLINT(google-explicit-constructor)
        : ScriptSource(std::string(source ? source : "")) {}

    /// The author's text, spelled out. Same as the implicit reading above;
    /// worth naming where a reader would otherwise wonder which half is which.
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

/**
 * @brief Builds one ScriptSource out of parts, keeping both halves in step.
 *
 * Some helpers do not merely forward the author's text — they COMPOSE with it.
 * `AssignmentExecutionHelper` turns a location and an expression into
 * `location = (expr);` and executes that as a script, and `<donedata>` glues
 * params together. Concatenating the two halves by hand at each such site is
 * how they would drift: the evaluated text would gain a piece the authored
 * text did not, and the next diagnostic would name a string nobody wrote.
 *
 * Evaluated text accumulates with evaluated text, authored with authored.
 * Punctuation that means the same thing in either language (`=`, `;`,
 * brackets) goes in through `add(const char *)` and lands in both.
 *
 * The language is fixed at construction and every composed part must match it:
 * a unit half in one language and half in another is not a thing an engine
 * could be handed, so it must not be a thing this can build.
 */
class ScriptSourceBuilder {
public:
    explicit ScriptSourceBuilder(ScriptLanguage language) : language_(language) {}

    /// Text that is identical in both languages — punctuation, an operator the
    /// lowering does not touch.
    ScriptSourceBuilder &add(const char *literal) {
        if (literal != nullptr) {
            text_ += literal;
            source_ += literal;
        }
        return *this;
    }

    /**
     * @brief A composed part: its evaluated half joins the evaluated text, its
     *        authored half joins the authored text.
     *
     * A part in the other language is dropped from BOTH halves rather than
     * silently mixed, and `hasMismatch()` says so — the caller is building
     * something an engine could not read, and finding that out from a
     * malformed script later is worse than finding it here.
     */
    ScriptSourceBuilder &add(const ScriptSource &part) {
        if (part.language() != language_) {
            mismatched_ = true;
            return *this;
        }
        text_ += part.text();
        source_ += part.source();
        return *this;
    }

    /// Whether a part in the wrong language was refused. A caller that can act
    /// on it should; one that cannot is at least not shipping a mixed string.
    bool hasMismatch() const {
        return mismatched_;
    }

    ScriptSource build() const {
        // Under ECMAScript the two halves are equal by construction — every
        // part has text == source, every literal lands in both — so either
        // spelling names the same string.
        return language_ == ScriptLanguage::Lua ? ScriptSource::lua(text_, source_) : ScriptSource::ecmascript(source_);
    }

private:
    ScriptLanguage language_;
    std::string text_;
    std::string source_;
    bool mismatched_ = false;
};

}  // namespace SCE
