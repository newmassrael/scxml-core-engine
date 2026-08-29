// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <cstdint>
#include <optional>
#include <string>

/// Opaque on purpose: the handle's shape belongs to `SceLowering.h`, and a
/// caller of this class never needs it. Declaring it here is what lets the
/// C surface stay in one translation unit.
struct SceLoweringScope;

namespace SCE {

/**
 * @file LoweringScope.h
 * @brief The names a session holds, as `sce-build`'s ECMAScript frontend sees them
 *
 * ## What a scope decides
 *
 * The frontend REFUSES any expression naming something its scope does not
 * declare, and that refusal is the whole selector. A scope with nothing in it
 * but the globals SCE installs answers only CLOSED expressions — `1 == '1'`,
 * `-7 % 3`, `Math.round(2.5)` — because those name nothing a session could
 * own. `a && b` names two things, and until the caller says what `a` and `b`
 * are, no parse can be trusted with them.
 *
 * So the scope is not configuration. It is the question, and it is now the
 * ONLY question: `EcmaScriptToLuaTransformer` used to answer whatever the
 * scope did not, and `docs/SCE_LUA_TRANSLATION_SEAM.md`'s `retire-rewriter`
 * row records that it no longer does. A name this scope has not been told
 * about is therefore an expression the engine refuses, which is why every
 * door that puts a name in a session's namespace has to come through
 * `LuaEngine::offerToScope`.
 *
 * ## Why a session owns one
 *
 * A run-time scope is the set of names the SESSION holds, which is exactly
 * what `<data id>` and a `<script>` chunk's top level introduce (W3C SCXML
 * §scxml-5.3, §scxml-5.8). It is therefore per-session state with a session's
 * lifetime, not a process-wide constant — two sessions of the same document
 * may legitimately differ, and one session's names must never answer for
 * another's.
 *
 * ## Growth is observable on purpose
 *
 * [`generation`] moves whenever a name is offered. A caller that CACHES a
 * lowering has cached an answer that depended on the scope, so it must be
 * able to tell that the scope has moved underneath it — otherwise the first
 * evaluation of an expression would pin the rewriter's answer for the life of
 * the session, and a `<script>` that ran afterwards could never change it.
 *
 * ## Refusal is an answer
 *
 * [`lowerValue`] returns nothing when the frontend will not lower the text.
 * That is normal, and it is now FINAL: while the rewriter stood, a refusal
 * meant "a caller falls back to whatever it did before", which is what let
 * the seam be adopted one expression class at a time. With nothing behind it,
 * a refusal is the engine's answer — `error.execution`, §scxml-5.9.1 — so
 * text this will not lower is text the datamodel does not evaluate, said out
 * loud rather than approximated.
 *
 * A build that links no frontend (`SCE_HAS_LOWERING_FFI` undefined — the wasm
 * build is one) refuses everything, by the same door. `LuaEngine` answers
 * that build's `acceptsLanguage(ECMAScript)` with false rather than accepting
 * a call it would refuse per expression.
 */
class LoweringScope {
public:
    LoweringScope();
    ~LoweringScope();

    // A scope is the identity of one session's name set. Copying one would
    // make two answers for a question that has one, so neither copy nor move
    // is offered; the owner holds it in place.
    LoweringScope(const LoweringScope &) = delete;
    LoweringScope &operator=(const LoweringScope &) = delete;
    LoweringScope(LoweringScope &&) = delete;
    LoweringScope &operator=(LoweringScope &&) = delete;

    /// Record one name, as a `<data id>` does.
    void declare(const std::string &name);

    /// Record what a chunk's top level introduces, as a `<script>` does.
    ///
    /// Only the top level, because only the top level reaches the datamodel.
    /// A chunk the frontend's parser refuses declares nothing — this is a
    /// name collector, not a second validator, and the expressions that would
    /// have named those variables simply keep the answer they had.
    void declareChunk(const std::string &source);

    /// The frontend's Lua for a value expression, or nothing if it refuses.
    std::optional<std::string> lowerValue(const std::string &source) const;

    /// The frontend's Lua for a statement sequence, or nothing if it refuses.
    ///
    /// A chunk brings its own names with it — `var` bindings are hoisted into
    /// the chunk's frame before anything resolves — so this asks LESS of the
    /// scope than [`lowerValue`] does. What it still asks the scope for is the
    /// names the chunk only READS: a `<data id>` the document declared, or a
    /// variable an earlier `<script>` introduced.
    std::optional<std::string> lowerScript(const std::string &source) const;

    /// How many times a name has been offered to this scope.
    ///
    /// A counter rather than a set comparison, and it counts OFFERS rather
    /// than additions: re-declaring a name it already holds still moves it.
    /// That over-counts, and over-counting costs one re-lowering while
    /// under-counting would serve a stale answer.
    uint64_t generation() const {
        return generation_;
    }

private:
    SceLoweringScope *scope_ = nullptr;
    uint64_t generation_ = 0;
};

}  // namespace SCE
