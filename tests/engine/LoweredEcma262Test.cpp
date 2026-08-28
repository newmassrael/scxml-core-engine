// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @brief A Lua-lowered C++ artifact, compiled and RUN, answers ECMA-262
 *
 * `docs/SCE_LUA_TRANSLATION_SEAM.md` ended with this as the open item: the
 * seam existed, the templates crossed it, `--script-engine lua -l cpp`
 * generated — *"and no gate configures `-DSCE_SCRIPT_ENGINE=lua` and compiles
 * one, so the ECMA-262 divergences this axis exists to close are not yet
 * measured through it."* Three comments in `tests/CMakeLists.txt` said the same
 * sentence from the other side. This is that gate.
 *
 * ## Which side of the seam this measures
 *
 * BUILD-TIME lowering. `tests/ecmascript/lua_engine_divergences.json` measures
 * the RUN-TIME rewriter (`EcmaScriptToLuaTransformer`), and the two are
 * different code paths reaching the same engine — so this file states which one
 * it is asking about rather than leaving a reader to infer it from the shared
 * engine.
 *
 * The population is that file all the same, because it is the set the axis
 * exists to close: every entry names an expression the runtime rewriter answers
 * differently from the language. `tools/generate_lowered_ecma262_fixture.py`
 * turns each entry into one state of an SCXML document, and this binary carries
 * that document generated TWICE:
 *
 *   - `ecma262_lowered`, generated with `--script-engine lua`. Its guards and
 *     `expr`s reach the engine as `ScriptSource::lua(...)` pairs the build-time
 *     frontend produced, so the rewriter is bypassed and every declared
 *     divergence has to answer ECMA-262.
 *   - `ecma262_source`, the same document generated the way C++ has always
 *     been generated. Its guards reach the same engine as the author's
 *     ECMAScript, the rewriter runs, and every declared divergence has to come
 *     back.
 *
 * The second is not a bonus case, it is the CONTROL, and it is why the first
 * one's green means something. A harness that compared nothing, or a fixture
 * whose cases had quietly stopped being asked, would report the lowered machine
 * perfect — and would report the source machine perfect too, where the
 * divergence list says it cannot be. One document, one comparison, two
 * artifacts, and the only difference between them is the codegen flag under
 * test.
 *
 * ## Why the answers come back as one JSON object
 *
 * §scxml-5.3 read accessors are typed from the authored literal, so a table
 * holding booleans, numbers and strings would need three of them and a case
 * whose answer changed type would read as ABSENT rather than as wrong. The
 * fixture records every answer into one `<data>` object and this reads it
 * through `answers()` — `JSON.stringify` in the engine, which is the one read
 * surface that carries a value's type. `docs/SCE_LUA_TRANSLATION_SEAM.md`
 * records why that expression survives either engine family: `JSON` is a real
 * table in the Lua runtime library, spelled the same as in ECMAScript.
 *
 * ## Why a `cond=` case answers 1, 2 or 3
 *
 * §scxml-5.9.1 makes a `cond` that raises evaluate to false. A boolean record
 * therefore cannot tell "the guard was false" from "the guard was refused", and
 * a case expecting `false` would pass on an expression the engine could not
 * even parse. So the fixture asks the guard AND its negation: 1 means the
 * source was truthy, 2 means its negation was, and 3 — the unguarded
 * fallthrough — means NEITHER held, which only an expression the engine refused
 * can produce. The verdict is on a positive guard and the fallthrough is a
 * failure, never a default.
 */

#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptSource.h"
#include <cstdint>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

#include "ecma262_lowered_sm.h"
#include "ecma262_source_sm.h"

// The whole point of this target is the engine selection, so a build that did
// not make it has nothing to say here. Registered only under the `lua`
// selection by `tests/CMakeLists.txt`; this is the second lock, so that a
// target list edited by hand cannot quietly compile this file into a QuickJS
// tree and report a green run about an engine it never reached.
#ifndef SCE_SCRIPT_ENGINE_LUA
#error "LoweredEcma262Test measures the lua selection; configure -DSCE_SCRIPT_ENGINE=lua"
#endif

namespace {

using nlohmann::json;

/// What a `cond=` case records. See the file comment: three values, because
/// §scxml-5.9.1 makes a refused guard indistinguishable from a false one.
constexpr int64_t COND_TRUE = 1;
constexpr int64_t COND_FALSE = 2;
constexpr int64_t COND_NEITHER = 3;

/// One expected answer, joined from the two shared tables.
struct Expected {
    /// `d<N>` — the key the fixture records into, and N is the index into the
    /// divergence list, which is what makes the population that file's.
    std::string key;
    /// `v<N>` — where a condition case also records its expression evaluated as
    /// a VALUE. Present means the engine could evaluate the text; absent means
    /// it refused. Never compared against `expect`: the shared table's answer
    /// for a condition case is the answer a guard gives.
    std::string probeKey;
    /// The ECMAScript the fixture asks, straight from the divergence entry.
    std::string source;
    /// The ECMA-262 clause the divergence entry cites.
    std::string clause;
    /// What closing it would take, per the divergence list's `needs`. Carried
    /// so a failure names the family rather than only the expression.
    std::string needs;
    /// True when the fixture asks it as a guard rather than as an `expr`.
    bool asCondition = false;
    /// The shared table's expectation, as JSON, so the comparison is by value
    /// and by type in one step.
    json expect;
};

json readJsonFile(const char *path, const char *what) {
    std::ifstream file(path);
    if (!file.is_open()) {
        // Not a throw: the floor below reports "0 case(s) joined", which says
        // the table was not read. A stack trace would not.
        ADD_FAILURE() << "cannot read " << what << " at " << path;
        return json::object();
    }
    json parsed;
    file >> parsed;
    return parsed;
}

/**
 * @brief Pair every divergence entry with the shared table's case.
 *
 * The same join `tools/generate_lowered_ecma262_fixture.py` makes, computed
 * here independently from the same two committed files. Deliberately not
 * imported from the generator and deliberately not emitted by it: a harness
 * reading the generator's own idea of the population would be the shape
 * `docs/SCE_LUA_TRANSLATION_SEAM.md` names — *"A gate whose two halves share a
 * source is not a gate."*
 */
std::vector<Expected> joinPopulation() {
    const json table = readJsonFile(SCE_ECMA262_CASES_PATH, "the shared ECMA-262 table");
    const json divergences = readJsonFile(SCE_LUA_DIVERGENCES_PATH, "the divergence list");

    std::map<std::string, const json *> byKey;
    if (table.contains("cases")) {
        for (const auto &entry : table.at("cases")) {
            byKey[entry.value("source", std::string{}) + "\x1f" + entry.value("clause", std::string{})] = &entry;
        }
    }

    std::vector<Expected> population;
    if (!divergences.contains("divergences")) {
        return population;
    }
    const auto &list = divergences.at("divergences");
    for (size_t n = 0; n < list.size(); ++n) {
        const json &entry = list.at(n);
        Expected expected;
        expected.key = "d" + std::to_string(n);
        expected.probeKey = "v" + std::to_string(n);
        expected.source = entry.value("source", std::string{});
        expected.clause = entry.value("clause", std::string{});
        expected.needs = entry.value("needs", std::string{});

        auto hit = byKey.find(expected.source + "\x1f" + expected.clause);
        if (hit == byKey.end()) {
            // The scoreboard gate already refuses a divergence naming no case
            // (`sce-build/tests/ecma262_scoreboard_contract.rs`). Refusing it
            // here too keeps this lane from silently shrinking its own
            // population when that one is not selected.
            ADD_FAILURE() << "divergence " << n << " [" << expected.source << "] (" << expected.clause
                          << ") names no case in the shared table, so this gate has nothing to ask for it";
            continue;
        }
        const json &testCase = *hit->second;
        expected.asCondition = testCase.value("form", std::string{}) == "condition";
        expected.expect = testCase.value("expect", json::object());
        if (expected.asCondition && !expected.expect.contains("bool")) {
            ADD_FAILURE() << "case [" << expected.source << "] is a condition but expects " << expected.expect.dump()
                          << "; a guard answers a boolean";
            continue;
        }
        population.push_back(std::move(expected));
    }
    return population;
}

/// The population, joined once. Read by both directions of the gate, so a
/// disagreement between them cannot come from having read the tables twice.
const std::vector<Expected> &population() {
    static const std::vector<Expected> joined = joinPopulation();
    return joined;
}

/**
 * @brief Run one generated machine and hand back what it recorded.
 *
 * Templated on the machine because the two artifacts under test are two
 * different types by construction — the fixture is generated under two names
 * so that two `_sm.h` headers, two namespaces and two policies can live in one
 * binary without an ODR clash.
 */
template <typename SM> json runMachine(const char *label) {
    SM machine;
    // §scxml-B-1: the generated machine takes its engine as a handle. The
    // provider's engine is a singleton, so this is a non-owning view of it —
    // the same aliasing shape `SimpleAotTest` uses.
    machine.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    machine.initialize();

    if (!machine.isInFinalState()) {
        ADD_FAILURE() << label
                      << ": the machine did not reach its final state, so the cases after the one it "
                         "stopped on were never asked";
    }

    auto recorded = machine.answers();
    if (!recorded.has_value()) {
        ADD_FAILURE() << label
                      << ": the `answers` datamodel object could not be read back — no engine, no session, "
                         "or the engine refused `JSON.stringify(answers)`";
        return json::object();
    }
    return json::parse(*recorded, nullptr, false);
}

/// What one case answered, reduced to the shape the expectation is in.
///
/// `std::nullopt` means the fixture recorded nothing for it, which is never a
/// pass: it says the state was not entered, or the assignment the state makes
/// was refused.
std::optional<json> recordedFor(const json &answers, const Expected &expected) {
    if (!answers.is_object() || !answers.contains(expected.key)) {
        return std::nullopt;
    }
    return answers.at(expected.key);
}

/// The sentinel the fixture parks in its probe variable before asking the
/// engine. §scxml-5.4 leaves an assignment's location unchanged when the
/// expression fails, so the sentinel surviving IS the engine's refusal. Must
/// match `PROBE_UNEVALUATED` in tools/generate_lowered_ecma262_fixture.py.
constexpr const char *PROBE_UNEVALUATED = "<unevaluated>";

/// Could the engine evaluate this case's expression at all?
///
/// Only a condition case carries the probe. For a value case the question is
/// already answered by whether `d<N>` was recorded, because a value case
/// records nothing else.
///
/// A probe that was never recorded at all is reported as "could not evaluate"
/// rather than as unknown: the recording assignment reads a bare variable, and
/// if even that failed the case has told us nothing about the expression.
bool engineCouldEvaluate(const json &answers, const Expected &expected) {
    if (!answers.is_object()) {
        return false;
    }
    if (!expected.asCondition) {
        return answers.contains(expected.key);
    }
    if (!answers.contains(expected.probeKey)) {
        return false;
    }
    const json &probe = answers.at(expected.probeKey);
    return !(probe.is_string() && probe.get<std::string>() == PROBE_UNEVALUATED);
}

/// Does one recorded answer agree with ECMA-262?
///
/// Numbers are compared as doubles because the two engine families hold them
/// differently and the shared table says so in as many words ("an engine may
/// hold it as an integer or a double"). Everything else is compared by type as
/// well as by value: `0` and `false` and `""` are three different answers, and
/// the truthiness family is exactly where a Lua-shaped rewriter confuses them.
bool agrees(const json &recorded, const Expected &expected) {
    if (expected.asCondition) {
        if (!recorded.is_number()) {
            return false;
        }
        const auto verdict = recorded.get<int64_t>();
        if (verdict == COND_NEITHER) {
            // Neither the guard nor its negation held. §scxml-5.9.1 makes a
            // raising `cond` false, so this is the engine having refused the
            // expression — never an answer about the language.
            return false;
        }
        const bool wanted = expected.expect.at("bool").get<bool>();
        return verdict == (wanted ? COND_TRUE : COND_FALSE);
    }
    if (expected.expect.contains("bool")) {
        return recorded.is_boolean() && recorded.get<bool>() == expected.expect.at("bool").get<bool>();
    }
    if (expected.expect.contains("number")) {
        return recorded.is_number() && recorded.get<double>() == expected.expect.at("number").get<double>();
    }
    if (expected.expect.contains("string")) {
        return recorded.is_string() && recorded.get<std::string>() == expected.expect.at("string").get<std::string>();
    }
    return false;
}

std::string describe(const std::optional<json> &recorded, const Expected &expected) {
    std::string shown = recorded.has_value() ? recorded->dump() : std::string{"<not recorded>"};
    if (expected.asCondition && recorded.has_value() && recorded->is_number()) {
        switch (recorded->get<int64_t>()) {
        case COND_TRUE:
            shown += " (the guard held)";
            break;
        case COND_FALSE:
            shown += " (its negation held)";
            break;
        case COND_NEITHER:
            shown += " (NEITHER held — the engine refused the expression)";
            break;
        default:
            break;
        }
    }
    return shown;
}

/// The floor. A table that shrank to nothing would score every engine
/// perfectly, which is the reason the two tables that feed this one carry the
/// same kind of floor.
constexpr size_t MIN_CASES = 20;

TEST(LoweredEcma262, ThePopulationIsTheDeclaredDivergenceSet) {
    const auto &cases = population();
    EXPECT_GE(cases.size(), MIN_CASES) << "only " << cases.size() << " case(s) joined from " << SCE_LUA_DIVERGENCES_PATH
                                       << " and " << SCE_ECMA262_CASES_PATH
                                       << ". This gate asks the declared divergences and nothing else, so a short join "
                                       << "is a smaller suite reported under the same name.";
}

TEST(LoweredEcma262, TheEngineThisBuildSelectedIsTheLuaOne) {
    auto &engine = ::SCE::ScriptEngineProvider::getScriptEngine();
    EXPECT_EQ(engine.nativeLanguage(), ::SCE::ScriptLanguage::Lua)
        << "the provider handed back a " << ::SCE::scriptLanguageName(engine.nativeLanguage())
        << " engine. Everything below is about what the lua selection answers, so on any other engine it would be "
           "measuring something else under this name.";
    EXPECT_TRUE(engine.acceptsLanguage(::SCE::ScriptLanguage::Lua))
        << "the selected engine refuses pre-lowered Lua, so the lowered artifact cannot have crossed the seam";
}

/// The measurement: build-time lowering answers the language.
TEST(LoweredEcma262, TheLoweredArtifactAnswersEveryDeclaredDivergence) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json answers = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");

    for (const auto &expected : cases) {
        const auto recorded = recordedFor(answers, expected);
        EXPECT_TRUE(recorded.has_value() && agrees(*recorded, expected))
            << "the lowered C++ artifact answered [" << expected.source << "] as " << describe(recorded, expected)
            << ", but ECMA-262 " << expected.clause << " says " << expected.expect.dump() << ".\n"
            << "  This is the point of --script-engine lua: the build-time frontend lowered that expression, so the\n"
            << "  runtime rewriter (needs: " << expected.needs << ") was never reached and the answer should be the\n"
            << "  language's. See docs/SCE_LUA_TRANSLATION_SEAM.md.";
    }
}

/// How many of the declared entries the SOURCE-passing artifact must get wrong.
///
/// A measurement, not a target. Measured 2026-08-29 on the first run of this
/// gate: 22 of the 23. The one that comes out right is named by the failure
/// message when this floor is missed, so nobody has to guess which.
///
/// Why a floor rather than "all of them", which is what this case first
/// asserted: the divergence list measures the ENGINE's answer, reached through
/// `GuardHelper::evaluateGuard`, and a generated `cond=` site is not that call.
/// §scxml-5.9.1 requires a guard the engine refused to evaluate as FALSE, so an
/// entry the rewriter cannot evaluate at all is observed as `false` at a
/// `cond=` site — and for an entry whose ECMA-262 answer IS false, the refusal
/// and the language coincide. `EngineRefusals` below is what tells those apart,
/// so the exception is attributed rather than exempted.
///
/// A floor that stops being met is a good red: it means the runtime rewriter
/// gained a case, and the list should shrink. Re-measure and lower it then.
constexpr size_t MIN_SOURCE_DIVERGENCES = 22;

/// The control: the same document, generated the way C++ has always been
/// generated, must NOT answer the language.
///
/// This is what makes the measurement above non-vacuous. A harness comparing
/// nothing, or a fixture whose cases had quietly stopped being asked, would
/// report the lowered machine perfect — and would report this one perfect too,
/// where the divergence list says it cannot be.
///
/// It also holds the direction the axis cares about: an answer the lowered
/// artifact loses would show up as a case this one gets RIGHT and the other
/// gets wrong, which is asserted per entry rather than counted.
TEST(LoweredEcma262, TheSourcePassingArtifactIsNotTheLoweredOne) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json lowered = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");
    const json source = runMachine<::SCE::Generated::ecma262_source::ecma262_source>("the source-passing artifact");

    size_t diverged = 0;
    std::vector<std::string> agreed;
    for (const auto &expected : cases) {
        const auto fromSource = recordedFor(source, expected);
        const bool sourceAgrees = fromSource.has_value() && agrees(*fromSource, expected);
        if (!sourceAgrees) {
            ++diverged;
        } else {
            const bool evaluable = engineCouldEvaluate(source, expected);
            agreed.push_back("[" + expected.source + "] (" + expected.clause + ", needs: " + expected.needs +
                             ") — the engine " +
                             (evaluable ? "evaluated the expression, so this is a real answer"
                                        : "REFUSED the expression, so §scxml-5.9.1's false is "
                                          "what a cond= site saw, not an answer about the language"));
        }

        // Per entry, and in the direction lowering must never lose: whatever
        // the source-passing artifact answers correctly, the lowered one must
        // answer correctly too.
        const auto fromLowered = recordedFor(lowered, expected);
        if (sourceAgrees) {
            EXPECT_TRUE(fromLowered.has_value() && agrees(*fromLowered, expected))
                << "[" << expected.source << "] is answered correctly WITHOUT lowering and wrongly WITH it ("
                << describe(fromLowered, expected) << "). Build-time lowering lost an answer the runtime rewriter "
                << "already had, which is a regression in the frontend, not a divergence.";
        }
    }

    EXPECT_GE(diverged, MIN_SOURCE_DIVERGENCES)
        << "only " << diverged << " of " << cases.size()
        << " declared divergences came back through the source-passing artifact (floor " << MIN_SOURCE_DIVERGENCES
        << "). The entries it answered correctly:\n  " <<
        [&agreed] {
            std::string text;
            for (const auto &line : agreed) {
                text += line + "\n  ";
            }
            return text;
        }()
        << "\n  Either the runtime rewriter gained those cases — in which case " << SCE_LUA_DIVERGENCES_PATH
        << " should shrink and this floor should be re-measured, the way that list's own header describes — or "
        << "this artifact was generated with --script-engine lua too, in which case the measurement above is "
        << "comparing the lowered machine against itself.";
}

/// The attribution the floor above would otherwise leave as a number.
///
/// `MIN_SOURCE_DIVERGENCES` allows some declared entries to come back correct
/// through a `cond=` site without lowering, and a bare allowance is an
/// exemption. This is the reason, stated as a predicate instead: an entry the
/// source-passing artifact answers correctly is only admissible when the engine
/// REFUSED the expression, so what the site observed was §scxml-5.9.1's
/// mandatory false and not the rewriter answering the language.
///
/// An entry that agrees AND was evaluable is a divergence the runtime rewriter
/// has actually repaired. That is a real finding and belongs in the list's own
/// two-directional contract — delete the entry — not in a tolerance here.
TEST(LoweredEcma262, EveryEntryTheSourceArtifactGetsRightIsAnEngineRefusal) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json answers = runMachine<::SCE::Generated::ecma262_source::ecma262_source>("the source-passing artifact");

    // The probe's own control. A probe stuck on one answer would make the
    // assertion below pass by measuring nothing — "everything was refused"
    // excuses every disagreement, and "nothing was refused" would fire on a
    // case that is genuinely a refusal. So the probe has to be shown to
    // DISTINGUISH, on this same run, before its verdict is used.
    size_t evaluable = 0;
    size_t refused = 0;
    for (const auto &expected : cases) {
        if (!expected.asCondition) {
            continue;
        }
        if (engineCouldEvaluate(answers, expected)) {
            ++evaluable;
        } else {
            ++refused;
        }
    }
    EXPECT_GT(evaluable, 0u) << "the refusal probe reported every condition case as refused, so it is not "
                                "distinguishing anything and the attribution below excuses whatever it is given";
    EXPECT_GT(refused, 0u) << "the refusal probe reported every condition case as evaluable, so it cannot be "
                              "reporting §scxml-5.9.1 refusals at all";

    for (const auto &expected : cases) {
        const auto recorded = recordedFor(answers, expected);
        if (!recorded.has_value() || !agrees(*recorded, expected)) {
            continue;
        }
        EXPECT_FALSE(engineCouldEvaluate(answers, expected))
            << "the source-passing artifact answered [" << expected.source << "] correctly (" << expected.expect.dump()
            << ") AND the engine could evaluate the expression, so this is the runtime "
            << "rewriter having gained the case rather than §scxml-5.9.1 turning a refusal into a false.\n"
            << "  " << SCE_LUA_DIVERGENCES_PATH << " still declares it a divergence (needs: " << expected.needs
            << "). Delete the entry — a repaired divergence that stays listed is what that file's header calls a\n"
            << "  case that 'starts agreeing', and it is meant to fail as loudly as one that starts disagreeing.";
    }
}

}  // namespace
