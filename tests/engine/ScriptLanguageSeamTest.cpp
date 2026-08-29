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
#include "common/AssignmentExecutionHelper.h"
#include "core/ForeachHelper.h"
#include "scripting/ScriptDialect.h"
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
 * @brief A `<data id>` declaration is what lets the frontend answer at all.
 *
 * The seam offers the author's ECMAScript to `sce-build`'s parser before the
 * rewriter, and the parser REFUSES anything naming a variable its scope does
 * not declare. So the scope is the selector, and `setVariable` — what a
 * `DataModelInitializer` calls for every `<data id>` (§scxml-5.3) — is the
 * half of it that no `<script>` supplies.
 *
 * `!a` with `a = 0` is the discriminator because the two answers differ by the
 * language rather than by a detail: ECMA-262 7.1.2 makes `ToBoolean(0)` false,
 * so `!a` is TRUE, while Lua counts 0 as truthy, so a rewritten `not a` is
 * FALSE. The shared ECMA-262 table cannot ask this question of this route —
 * every one of its setups arrives through `executeScript` — so without this
 * test the `<data id>` half of the scope has no oracle at all.
 */
TEST(ScriptLanguageSeam, ADataItemDeclarationIsWhatLetsTheFrontendAnswer) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_data_id_reaches_the_scope";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    ASSERT_TRUE(engine.setVariable(sessionId, "a", ScriptValue{static_cast<int64_t>(0)}).get().isSuccess());

    auto answered = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("!a")).get();
    ASSERT_TRUE(answered.isSuccess()) << answered.getErrorMessage();
    ASSERT_TRUE(std::holds_alternative<bool>(answered.getInternalValue()))
        << "`!a` answered something that is not a boolean, so this test can no longer tell the "
           "two lowerings apart";
    EXPECT_TRUE(std::get<bool>(answered.getInternalValue()))
        << "`!a` with `a = 0` answered false, which is Lua's reading of `not a`. ECMA-262 7.1.2 "
           "makes ToBoolean(0) false and so `!a` true, so this says the declaration never reached "
           "the frontend's scope and the expression was rewritten instead of parsed.";

    engine.destroySession(sessionId);
}

/**
 * @brief A lowering cached against an older scope is not reused.
 *
 * The per-session expression cache is keyed on the AUTHOR'S text, and the same
 * text lowers differently as the scope grows: `a && b` is refused while `b` is
 * unknown and answered once it is declared. So a cache that ignored the scope
 * would pin whichever answer came first for the life of the session, and the
 * later declaration could never reach it — a correctness bug wearing the shape
 * of a caching win.
 *
 * The first evaluation is the control as well as the setup. If it already
 * answered 0, the scope was never the selector and the assertion below would
 * pass without a re-lowering having happened.
 */
TEST(ScriptLanguageSeam, ALoweringCachedAgainstAnOlderScopeIsNotReused) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_cache_follows_the_scope";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    ASSERT_TRUE(engine.setVariable(sessionId, "a", ScriptValue{static_cast<int64_t>(0)}).get().isSuccess());

    // `b` is undeclared, so the frontend refuses the whole expression and the
    // rewriter answers it: Lua's `a and b` counts 0 as truthy and yields `b`,
    // which is nil.
    auto before = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("a && b")).get();
    const bool alreadyRight = before.isSuccess() && std::holds_alternative<int64_t>(before.getInternalValue()) &&
                              std::get<int64_t>(before.getInternalValue()) == 0;
    ASSERT_FALSE(alreadyRight) << "`a && b` was already answered correctly with `b` undeclared, so this test "
                                  "cannot tell a re-lowering from a cache hit";

    ASSERT_TRUE(engine.setVariable(sessionId, "b", ScriptValue{static_cast<int64_t>(2)}).get().isSuccess());

    auto after = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("a && b")).get();
    ASSERT_TRUE(after.isSuccess()) << after.getErrorMessage();
    EXPECT_EQ(asWholeNumber(after.getInternalValue()), 0)
        << "ECMA-262 13.13.1 yields the LEFT operand when it is falsy, so `a && b` is 0. The same "
           "text answered differently before `b` was declared, so answering it the same way twice "
           "means the chunk compiled against the older scope was reused.";

    engine.destroySession(sessionId);
}

/**
 * @brief The same, one grammar up: a `<script>` chunk follows the scope too.
 *
 * A chunk's lowering depends on the scope exactly as an expression's does, and
 * `scriptExecCache` is keyed on the author's TEXT — so an answer that outlived
 * the scope it was made against would be served forever under the same key.
 *
 * A chunk hoists its own `var` bindings, which is why the discriminator has to
 * be a name the chunk only READS: `b` is what the scope decides, and `r` is
 * the chunk's own.
 *
 * ⚠ **What this case can observe narrowed when the rewriter was retired, and
 * saying so is the point.** It used to compare two SUCCESSFUL answers: the
 * frontend refused the chunk, `EcmaScriptToLuaTransformer` answered it wrongly
 * (Lua's `a and b` counts 0 as truthy and yields `b`, which is nil), and a
 * cache hit was visible as that wrong answer surviving. With no second
 * translator behind the refusal, the frontend's scope-dependence has exactly
 * two outcomes — refused, or lowered — so a refusal is what the first
 * execution now produces, and what must not be remembered. Both directions
 * still fail loudly: a refusal that is cached leaves the second execution
 * refusing, and a scope that never hears about `b` never lets it succeed at
 * all. A two-way form would need a name the frontend lowers DIFFERENTLY rather
 * than not at all — an author `<data id>` shadowing an installed global is the
 * only such shape, and it is not what this case is about.
 */
TEST(ScriptLanguageSeam, AChunkRefusedAgainstAnOlderScopeIsAskedAgain) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_script_cache_follows_the_scope";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    ASSERT_TRUE(engine.setVariable(sessionId, "a", ScriptValue{static_cast<int64_t>(0)}).get().isSuccess());

    // `b` is undeclared, so the frontend refuses the whole chunk — and the
    // engine refuses with it rather than handing the text to a pass that
    // cannot read it.
    const SCE::ScriptSource chunk = SCE::ScriptSource::ecmascript("var r = a && b;");
    ASSERT_FALSE(engine.executeScript(sessionId, chunk).get().isSuccess())
        << "`b` is undeclared, so this chunk cannot be lowered and the first execution is the "
           "control: if it already succeeded, the assertion below could not tell a re-lowering "
           "from a cache hit";

    ASSERT_TRUE(engine.setVariable(sessionId, "b", ScriptValue{static_cast<int64_t>(2)}).get().isSuccess());

    ASSERT_TRUE(engine.executeScript(sessionId, chunk).get().isSuccess())
        << "the same chunk text was refused before `b` was declared. Refusing it again means the "
           "refusal was remembered under a key that does not carry the scope it was made against";
    auto after = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("r")).get();
    ASSERT_TRUE(after.isSuccess()) << after.getErrorMessage();
    EXPECT_EQ(asWholeNumber(after.getInternalValue()), 0)
        << "ECMA-262 13.13.1 yields the LEFT operand when it is falsy, so `r` is 0";

    engine.destroySession(sessionId);
}

/**
 * @brief An `<assign>` to a member location reaches the frontend.
 *
 * This is the route a DOCUMENT takes, and it is not the route a direct
 * `evaluateExpression` takes. `AssignmentExecutionHelper` sends a location that
 * is not a bare identifier down its complex path, which builds
 * `<location> = (<expr>);` and runs it as a SCRIPT — so the seam has to be on
 * `loweredScriptOf`, and while it was only on `loweredTextOf` every such
 * assignment silently fell back to the rewriter.
 *
 * That was not hypothetical. Measured 2026-08-29 on the `ecma262-lowered-cpp`
 * lane: `source-wrong=14` against a direct-evaluate suite reporting zero, on
 * one shared table, because that fixture records all 98 of its answers with
 * `<assign location="answers.dNN">` and every one of them took this path.
 *
 * `5 ^ 3` is the discriminator because the two readings differ by the LANGUAGE
 * rather than by a detail: ECMA-262 13.12 makes `^` bitwise XOR, so the answer
 * is 6, while Lua's `^` is exponentiation and answers 125. The rewriter leaves
 * `^` to Lua — it parenthesises bitwise operands and nothing more — so 125 is
 * precisely "the frontend was never asked".
 */
TEST(ScriptLanguageSeam, AMemberLocationAssignmentReachesTheFrontend) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_member_assign_reaches_the_frontend";
    ASSERT_TRUE(engine.createSession(sessionId, ""));

    // `<data id="answers" expr="{}"/>`, by the route a document takes: the
    // chunk runs, and §scxml-5.8 declares what its top level introduced.
    ASSERT_TRUE(engine.executeScript(sessionId, SCE::ScriptSource::ecmascript("var answers = {};")).get().isSuccess());

    bool raised = false;
    ASSERT_TRUE(SCE::AssignmentExecutionHelper::executeAssignment(
        engine, sessionId, SCE::ScriptSource::ecmascript("answers.d53"), SCE::ScriptSource::ecmascript("5 ^ 3"),
        [&raised](const std::string &) { raised = true; }))
        << "the complex-path assignment failed outright";
    ASSERT_FALSE(raised);

    auto answer = engine.evaluateExpression(sessionId, SCE::ScriptSource::ecmascript("answers.d53")).get();
    ASSERT_TRUE(answer.isSuccess()) << answer.getErrorMessage();
    EXPECT_EQ(asWholeNumber(answer.getInternalValue()), 6)
        << "ECMA-262 13.12 makes `5 ^ 3` bitwise XOR, which is 6. Lua reads `^` as exponentiation "
           "and answers 125, and the rewriter leaves `^` to Lua — so anything but 6 says this "
           "assignment never reached the frontend, and the seam is missing from the script path "
           "again.";

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

// ===========================================================================
// Composition — the half of the seam the helpers needed
//
// The helpers between the templates and the engine do not only forward the
// author's expression. Two of them GLUE onto it: `AssignmentExecutionHelper`
// builds `location = (expr);` and runs it, and `ForeachHelper` asks
// `(array) instanceof Array` before iterating. Glue written as ECMAScript
// around a pre-lowered Lua expression is neither language, and on a
// pre-lowered path there is no rewriter left to repair it.
// ===========================================================================

/**
 * @brief A composed unit keeps its two halves in step.
 *
 * The evaluated text gains the lowered part, the authored text gains the
 * author's — and the literal glue lands in both. If the builder pushed one
 * part into both halves the diagnostic would quote a string nobody wrote,
 * which is the failure the two-string seam exists to prevent.
 */
TEST(ScriptLanguageSeam, ComposingKeepsTheEvaluatedAndAuthoredHalvesApart) {
    const SCE::ScriptSource lowered = SCE::ScriptSource::lua("arr[1]", "arr[0]");
    const SCE::ScriptSource composed =
        SCE::ScriptSourceBuilder(SCE::ScriptLanguage::Lua).add("x = (").add(lowered).add(");").build();

    EXPECT_EQ(composed.language(), SCE::ScriptLanguage::Lua);
    EXPECT_EQ(composed.text(), "x = (arr[1]);") << "the evaluated half did not take the LOWERED part";
    EXPECT_EQ(composed.source(), "x = (arr[0]);") << "the authored half did not take the AUTHOR'S part";
}

/// A part in the other language is refused rather than mixed in.
TEST(ScriptLanguageSeam, ComposingRefusesAPartInTheOtherLanguage) {
    SCE::ScriptSourceBuilder builder(SCE::ScriptLanguage::Lua);
    builder.add("x = ").add(SCE::ScriptSource::ecmascript("arr[0]"));

    EXPECT_TRUE(builder.hasMismatch()) << "an ECMAScript part was accepted into a Lua unit";
    EXPECT_EQ(builder.build().text(), "x = ") << "the mismatched part was mixed into the evaluated text anyway";
}

#if defined(SCE_ENABLE_LUA) && defined(SCE_ENABLE_QUICKJS)

/**
 * @brief The two spellings of one question answer the same thing.
 *
 * This is the case that makes the dialect table more than a lookup: each
 * composed question is asked of BOTH engines, in each engine's own spelling,
 * about the same value — so a Lua spelling that does not exist, or exists and
 * means something else, disagrees with the ECMAScript one that W3C defines.
 *
 * Measured against the runtime this backend loads: `_isArray`, `_typeof` and
 * `_scxml_index` are `ecma_semantics.lua`, `JSON.stringify` is
 * `json_builtins.lua`, and `#` is Lua's own.
 */
TEST(ScriptLanguageSeam, EachComposedQuestionMeansTheSameInBothLanguages) {
    auto &lua = SCE::LuaEngine::instance();
    auto &quickjs = SCE::JSEngine::instance();
    ASSERT_TRUE(quickjs.initialize());

    const std::string luaSession = "seam_dialect_lua";
    const std::string jsSession = "seam_dialect_js";
    ASSERT_TRUE(lua.createSession(luaSession, ""));
    ASSERT_TRUE(quickjs.createSession(jsSession, ""));

    // The same array, each side written in its own language — this is the
    // fixture, not the measurement.
    ASSERT_TRUE(lua.executeScript(luaSession, SCE::ScriptSource::lua(ARRAY_SETUP, ARRAY_SETUP)).get().isSuccess());
    ASSERT_TRUE(quickjs.executeScript(jsSession, "arr = [10, 20, 30]").get().isSuccess());

    // The array expression, tagged as each engine will receive it.
    const SCE::ScriptSource asLua = SCE::ScriptSource::lua("arr", "arr");
    const SCE::ScriptSource asSource = SCE::ScriptSource::ecmascript("arr");

    // isArray — `_isArray(arr)` against `(arr) instanceof Array`
    auto luaIsArray = lua.evaluateExpression(luaSession, SCE::ScriptDialect::isArray(asLua)).get();
    auto jsIsArray = quickjs.evaluateExpression(jsSession, SCE::ScriptDialect::isArray(asSource)).get();
    ASSERT_TRUE(luaIsArray.isSuccess()) << "the Lua spelling of isArray did not evaluate: "
                                        << luaIsArray.getErrorMessage();
    ASSERT_TRUE(jsIsArray.isSuccess()) << jsIsArray.getErrorMessage();
    EXPECT_EQ(luaIsArray.getValue<bool>(), true);
    EXPECT_EQ(luaIsArray.getValue<bool>(), jsIsArray.getValue<bool>()) << "the two spellings of isArray disagree";

    // lengthOf — `#(arr)` against `(arr).length`
    auto luaLength = lua.evaluateExpression(luaSession, SCE::ScriptDialect::lengthOf(asLua)).get();
    auto jsLength = quickjs.evaluateExpression(jsSession, SCE::ScriptDialect::lengthOf(asSource)).get();
    ASSERT_TRUE(luaLength.isSuccess()) << "the Lua spelling of lengthOf did not evaluate: "
                                       << luaLength.getErrorMessage();
    ASSERT_TRUE(jsLength.isSuccess()) << jsLength.getErrorMessage();
    EXPECT_EQ(asWholeNumber(luaLength.getInternalValue()), 3);
    EXPECT_EQ(asWholeNumber(luaLength.getInternalValue()), asWholeNumber(jsLength.getInternalValue()))
        << "the two spellings of lengthOf disagree";

    // elementAt — counted from 0 in BOTH spellings; `_scxml_index` does the
    // shift to Lua's 1-based storage itself, and a caller that pre-shifted
    // would move the element twice.
    auto luaFirst = lua.evaluateExpression(luaSession, SCE::ScriptDialect::elementAt(asLua, 0)).get();
    auto jsFirst = quickjs.evaluateExpression(jsSession, SCE::ScriptDialect::elementAt(asSource, 0)).get();
    ASSERT_TRUE(luaFirst.isSuccess()) << "the Lua spelling of elementAt did not evaluate: "
                                      << luaFirst.getErrorMessage();
    ASSERT_TRUE(jsFirst.isSuccess()) << jsFirst.getErrorMessage();
    EXPECT_EQ(asWholeNumber(luaFirst.getInternalValue()), 10) << "element 0 is not the author's first element";
    EXPECT_EQ(asWholeNumber(luaFirst.getInternalValue()), asWholeNumber(jsFirst.getInternalValue()))
        << "the two spellings of elementAt disagree";

    // typeOf — `_typeof(x)` against `typeof (x)`
    auto luaType = lua.evaluateExpression(luaSession, SCE::ScriptDialect::typeOf(asLua)).get();
    auto jsType = quickjs.evaluateExpression(jsSession, SCE::ScriptDialect::typeOf(asSource)).get();
    ASSERT_TRUE(luaType.isSuccess()) << "the Lua spelling of typeOf did not evaluate: " << luaType.getErrorMessage();
    ASSERT_TRUE(jsType.isSuccess()) << jsType.getErrorMessage();
    EXPECT_EQ(luaType.getValue<std::string>(), "object");
    EXPECT_EQ(luaType.getValue<std::string>(), jsType.getValue<std::string>())
        << "the two spellings of typeOf disagree";

    // stringify — the one wrapper that is the same text in both languages,
    // because `JSON` is a real Lua table here.
    auto luaJson = lua.evaluateExpression(luaSession, SCE::ScriptDialect::stringify(asLua)).get();
    auto jsJson = quickjs.evaluateExpression(jsSession, SCE::ScriptDialect::stringify(asSource)).get();
    ASSERT_TRUE(luaJson.isSuccess()) << "the Lua spelling of stringify did not evaluate: " << luaJson.getErrorMessage();
    ASSERT_TRUE(jsJson.isSuccess()) << jsJson.getErrorMessage();
    EXPECT_EQ(luaJson.getValue<std::string>(), jsJson.getValue<std::string>())
        << "the two spellings of stringify disagree";

    lua.destroySession(luaSession);
    quickjs.destroySession(jsSession);
}

#endif  // SCE_ENABLE_LUA && SCE_ENABLE_QUICKJS

#ifdef SCE_ENABLE_LUA

/**
 * @brief A helper on the generated path takes a tagged expression end to end.
 *
 * `ForeachHelper::evaluateForeachArray` is what both `<foreach>` entry points
 * the C++ templates emit go through. What this measures is that the helper
 * carries a `ScriptSource` from the template's side to the engine and back:
 * the same call with the same array answers the same three elements whichever
 * language the expression is tagged as.
 *
 * ⚠ It does NOT measure the `isArray` probe that helper composes, and saying
 * so is the point. Measured 2026-08-28: replacing the probe with the
 * ECMAScript-only spelling leaves this case GREEN, because the probe sits
 * behind a short-circuit that a well-formed array never reaches —
 * `arrayResult.isArray()` is already true for a Lua sequence, and for a keyed
 * table the probe answers false either way (a Lua syntax error and an honest
 * `false` are the same outcome here). The probe's language-correctness is
 * therefore only observable in `EachComposedQuestionMeansTheSameInBothLanguages`,
 * which asks it directly of both engines — and that case does go red.
 */
TEST(ScriptLanguageSeam, ForeachCarriesATaggedArrayExpressionEndToEnd) {
    auto &engine = SCE::LuaEngine::instance();
    const std::string sessionId = "seam_foreach_lowered_array";
    ASSERT_TRUE(engine.createSession(sessionId, ""));
    ASSERT_TRUE(engine.executeScript(sessionId, SCE::ScriptSource::lua(ARRAY_SETUP, ARRAY_SETUP)).get().isSuccess());

    auto values =
        SCE::Core::ForeachHelper::evaluateForeachArray(engine, sessionId, SCE::ScriptSource::lua("arr", "arr"));
    ASSERT_TRUE(values.has_value()) << "the helper rejected a pre-lowered array expression outright";
    EXPECT_EQ(values->size(), 3u) << "the lowered tag did not reach the engine intact";

    // The other tag, same array, same answer: the helper is carrying the
    // ScriptSource rather than reading one half of it.
    auto asSource =
        SCE::Core::ForeachHelper::evaluateForeachArray(engine, sessionId, SCE::ScriptSource::ecmascript("arr"));
    ASSERT_TRUE(asSource.has_value()) << "the ECMAScript path stopped working";
    EXPECT_EQ(asSource->size(), values->size()) << "the two tags disagree about the same array";

    engine.destroySession(sessionId);
}

#endif  // SCE_ENABLE_LUA
