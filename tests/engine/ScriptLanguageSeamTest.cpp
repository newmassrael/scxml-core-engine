// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief The engine boundary carries a language, and two strings
 *
 * `datamodel="ecmascript"` is a claim about a language, and four of SCE's six
 * backends already keep it by lowering the author's ECMAScript to Lua at build
 * time. C++ does not: its generated code hands the engine the author's source
 * and a runtime rewriter translates on the spot, which is where the ECMA-262
 * divergences live (docs/SCE_LUA_TRANSLATION_SEAM.md).
 *
 * Moving C++ across needs an engine that can be handed text that is ALREADY
 * Lua. That is what this file measures, and the three things it measures are
 * the three the seam document says a one-string signature would lose:
 *
 *  1. **The rewrite is skipped, and skipping it is observable.** Handing the
 *     same text as Lua and as ECMAScript must answer differently, because the
 *     rewriter shifts a literal index by one. Asserting only the Lua half
 *     would pass against an engine that ignored the tag entirely.
 *  2. **The other four steps are NOT skipped.** The §scxml-5.9
 *     undeclared-variable check runs on the LOWERED text and reports the
 *     AUTHOR'S — that message rides out on `_event.data` of `error.execution`,
 *     so an entry point holding one string would name a language nobody wrote.
 *  3. **An engine handed a language it does not evaluate refuses.** QuickJS
 *     given Lua is the case: the text is well-formed, it is simply not
 *     ECMAScript, and trying it would produce a syntax error about someone
 *     else's language.
 *
 * `LuaEngine` and `JSEngine` are named here rather than reached through
 * `ScriptEngineProvider`, for the reason `DomReadSurfaceTest` next door
 * records: the provider is a compile-time choice and no gate configures
 * `-DSCE_SCRIPT_ENGINE=lua`, so a seam measured only through the provider
 * would have its Lua half compiled by every build and run by none.
 */

#include "SCXMLTypes.h"
#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptSource.h"
#include <cstdint>
#include <gtest/gtest.h>
#include <string>
#include <variant>

#ifdef SCE_ENABLE_LUA
#include "scripting/LuaEngine.h"
#endif

#ifdef SCE_ENABLE_QUICKJS
#include "scripting/JSEngine.h"
#endif

namespace {

/// The array the index cases read. Written as Lua because it is setup, not a
/// case: handing it over pre-lowered keeps the fixture from depending on the
/// very rewriter these tests are about.
constexpr const char *ARRAY_SETUP = "arr = { 10, 20, 30 }";

/// What `EcmaScriptToLuaTransformer::transformArrayIndexing` makes of
/// `arr[0]`: ECMAScript arrays are 0-based and Lua tables are 1-based, so the
/// author's first element is `arr[1]` once lowered.
constexpr const char *AUTHORED_FIRST_ELEMENT = "arr[0]";
constexpr const char *LOWERED_FIRST_ELEMENT = "arr[1]";

/// A whole number may come back as either of Lua 5.4's two numeric types.
int64_t asWholeNumber(const ::ScriptValue &value) {
    if (std::holds_alternative<int64_t>(value)) {
        return std::get<int64_t>(value);
    }
    if (std::holds_alternative<double>(value)) {
        return static_cast<int64_t>(std::get<double>(value));
    }
    return -1;
}

}  // namespace

#ifdef SCE_ENABLE_LUA

/**
 * @brief Lowered text is evaluated as given, and the proof is the difference.
 *
 * One string, two languages, two answers. `arr[1]` as already-lowered Lua is
 * the author's FIRST element; the same characters as ECMAScript are the
 * author's second, because the rewriter shifts the literal index. If the
 * engine ignored the tag both would answer 20 — the double shift the seam
 * document names as "an off-by-one with no diagnostic, which is worse than the
 * divergences this change exists to remove".
 */
TEST(ScriptLanguageSeam, LoweredLuaIsNotRewrittenAgain) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_no_second_rewrite";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto setup = engine.executeScript(sessionId, SCE::ScriptSource::lua(ARRAY_SETUP, ARRAY_SETUP)).get();
    ASSERT_TRUE(setup.isSuccess()) << setup.getErrorMessage();

    auto lowered =
        engine.evaluateExpression(sessionId, SCE::ScriptSource::lua(LOWERED_FIRST_ELEMENT, AUTHORED_FIRST_ELEMENT))
            .get();
    ASSERT_TRUE(lowered.isSuccess()) << lowered.getErrorMessage();
    EXPECT_EQ(asWholeNumber(lowered.getInternalValue()), 10)
        << "'" << LOWERED_FIRST_ELEMENT << "' handed over as already-lowered Lua was rewritten a second time; "
        << "the index the build-time frontend had already made 1-based got shifted again";

    // The control that keeps the assertion above from passing vacuously: the
    // rewriter demonstrably DOES run on the other tag, so skipping it is a
    // real bypass rather than a no-op.
    auto asSource = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript(LOWERED_FIRST_ELEMENT)).get();
    ASSERT_TRUE(asSource.isSuccess()) << asSource.getErrorMessage();
    EXPECT_EQ(asWholeNumber(asSource.getInternalValue()), 20)
        << "the ECMAScript path did not rewrite '" << LOWERED_FIRST_ELEMENT
        << "', so this file cannot tell a skipped rewrite from an absent one";

    engine.destroySession(sessionId);
}

/**
 * @brief The two answers stay two answers inside one session.
 *
 * The per-session fast paths are keyed on the INCOMING text, which one
 * language's lowering can make ambiguous: `arr[1]` means two different chunks
 * depending on the tag it arrived with. A cache that answered the first
 * caller's chunk to the second would reintroduce the same silent off-by-one
 * from the other direction.
 */
TEST(ScriptLanguageSeam, TheFastPathDoesNotConfuseTheTwoLanguages) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_cache_is_language_keyed";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto setup = engine.executeScript(sessionId, SCE::ScriptSource::lua(ARRAY_SETUP, ARRAY_SETUP)).get();
    ASSERT_TRUE(setup.isSuccess()) << setup.getErrorMessage();

    // ECMAScript first, so its chunk is the one already in the cache when the
    // Lua-tagged call arrives with the same key.
    ASSERT_EQ(asWholeNumber(engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript(LOWERED_FIRST_ELEMENT))
                                .get()
                                .getInternalValue()),
              20);
    EXPECT_EQ(
        asWholeNumber(
            engine.evaluateExpression(sessionId, SCE::ScriptSource::lua(LOWERED_FIRST_ELEMENT, AUTHORED_FIRST_ELEMENT))
                .get()
                .getInternalValue()),
        10)
        << "the Lua-tagged call was answered with the chunk compiled for the ECMAScript-tagged one";

    // And the same in the other order, so this passes for the reason it
    // claims rather than because one direction happens to recompile.
    EXPECT_EQ(asWholeNumber(engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript(LOWERED_FIRST_ELEMENT))
                                .get()
                                .getInternalValue()),
              20)
        << "the ECMAScript-tagged call was answered with the chunk compiled for the Lua-tagged one";

    engine.destroySession(sessionId);
}

/**
 * @brief §scxml-5.9's ReferenceError names what the author wrote.
 *
 * The check runs on the lowered text — that is the only text an engine can
 * resolve identifiers in — and the message is built from the source. This is
 * the whole reason the seam carries two strings: the text travels out on
 * `_event.data` of `error.execution`, where naming `nosuchvar[1]` would tell
 * whoever wrote `nosuchvar[0]` about a line they never wrote.
 */
TEST(ScriptLanguageSeam, TheReferenceErrorNamesTheAuthorsTextNotTheLoweredOne) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_diagnostic_names_source";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto result = engine.evaluateExpression(sessionId, SCE::ScriptSource::lua("nosuchvar[1]", "nosuchvar[0]")).get();
    ASSERT_TRUE(result.isError()) << "an undeclared variable answered instead of raising ReferenceError";

    const std::string message = result.getErrorMessage();
    EXPECT_NE(message.find("nosuchvar[0]"), std::string::npos)
        << "the diagnostic does not name the author's expression: " << message;
    EXPECT_EQ(message.find("nosuchvar[1]"), std::string::npos)
        << "the diagnostic names the lowered Lua, a language the author never wrote: " << message;

    engine.destroySession(sessionId);
}

/// The engine that owns an adapter for the other language says so.
TEST(ScriptLanguageSeam, LuaEngineIsNativeLuaAndAdaptsEcmaScript) {
    auto &engine = SCE::LuaEngine::instance();
    EXPECT_EQ(engine.nativeLanguage(), SCE::ScriptLanguage::Lua);
    EXPECT_TRUE(engine.acceptsLanguage(SCE::ScriptLanguage::Lua));
    EXPECT_TRUE(engine.acceptsLanguage(SCE::ScriptLanguage::ECMAScript))
        << "EcmaScriptToLuaTransformer is this engine's input adapter, so refusing ECMAScript "
        << "would refuse every C++ machine generated today";
}

#endif  // SCE_ENABLE_LUA

#ifdef SCE_ENABLE_QUICKJS

/**
 * @brief QuickJS handed Lua refuses, and says which two languages.
 *
 * The refusal is the half of the seam that makes `--script-engine lua` safe to
 * add: once generated code can emit Lua, an engine that cannot read it has to
 * say so rather than report a syntax error in a language the author never
 * chose. The shape follows `sce-build`'s mesh-rpc refusal — name what is
 * missing, and name what would satisfy it.
 */
TEST(ScriptLanguageSeam, QuickJsRefusesLoweredLuaRatherThanTryingIt) {
    auto &engine = SCE::JSEngine::instance();
    ASSERT_TRUE(engine.initialize());
    const std::string sessionId = "seam_quickjs_refusal";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    auto refused = engine.evaluateExpression(sessionId, SCE::ScriptSource::lua("arr[1]", "arr[0]")).get();
    ASSERT_TRUE(refused.isError()) << "an ECMAScript engine evaluated Lua instead of refusing it";

    const std::string message = refused.getErrorMessage();
    EXPECT_NE(message.find("ecmascript"), std::string::npos)
        << "the refusal does not name the language this engine evaluates: " << message;
    EXPECT_NE(message.find("lua"), std::string::npos)
        << "the refusal does not name the language it was handed: " << message;
    EXPECT_NE(message.find("arr[0]"), std::string::npos)
        << "the refusal does not name the author's expression, so nobody can find it: " << message;

    // The control: the refusal is about the LANGUAGE, not about this engine
    // failing everything. The same session evaluates the author's ECMAScript.
    auto accepted = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("1 + 1")).get();
    EXPECT_TRUE(accepted.isSuccess()) << "the engine refused ECMAScript too, so the case above proves nothing: "
                                      << accepted.getErrorMessage();

    engine.destroySession(sessionId);
}

/// The engine with no adapter says so, which is what makes the refusal happen.
TEST(ScriptLanguageSeam, QuickJsIsNativeEcmaScriptAndAdaptsNothing) {
    auto &engine = SCE::JSEngine::instance();
    EXPECT_EQ(engine.nativeLanguage(), SCE::ScriptLanguage::ECMAScript);
    EXPECT_TRUE(engine.acceptsLanguage(SCE::ScriptLanguage::ECMAScript));
    EXPECT_FALSE(engine.acceptsLanguage(SCE::ScriptLanguage::Lua));
}

#endif  // SCE_ENABLE_QUICKJS

/**
 * @brief The tag the engine reports is the engine this build selected.
 *
 * `nativeLanguage()` is the engine-side mirror of the manifest's
 * `script_engine_language`, and the manifest field was wrong for C++ and
 * Kotlin for exactly as long as nothing compared it with anything. This
 * compares it with `SCE_SCRIPT_ENGINE`, so a provider wired to one engine
 * while reporting another cannot pass.
 */
TEST(ScriptLanguageSeam, TheProvidersEngineReportsTheLanguageTheBuildSelected) {
    auto &engine = SCE::ScriptEngineProvider::getScriptEngine();
    const std::string selected = SCE::ScriptEngineProvider::getEngineId();

    if (selected == "lua") {
        EXPECT_EQ(engine.nativeLanguage(), SCE::ScriptLanguage::Lua);
    } else if (selected == "quickjs") {
        EXPECT_EQ(engine.nativeLanguage(), SCE::ScriptLanguage::ECMAScript);
    } else {
        FAIL() << "SCE_SCRIPT_ENGINE=" << selected << " has no language in this test's table, so the seam's tag "
               << "is unmeasured for the engine this build actually runs";
    }

    // Whatever the selection, the engine must accept what it says it speaks.
    EXPECT_TRUE(engine.acceptsLanguage(engine.nativeLanguage()));
}
