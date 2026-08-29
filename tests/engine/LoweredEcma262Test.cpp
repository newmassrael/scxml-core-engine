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
 * BUILD-TIME lowering, and it is now a two-directional contract rather than a
 * measurement. There are exactly two code paths from `datamodel="ecmascript"`
 * into the same Lua engine, and `tests/ecmascript/lua_engine_divergences.json`
 * names both in each entry's `diverges_on`:
 *
 *   - `runtime-rewriter` — the artifact hands the engine the AUTHOR'S
 *     ECMAScript and the engine lowers it on the spot. The path keeps the
 *     rewriter's name because that is what used to do the lowering; since
 *     `EcmaScriptToLuaTransformer` was retired the frontend does, at run
 *     time. `ecmascript_semantics_test` holds that path in both directions.
 *   - `build-time-lowering` — `sce-build`'s frontend emitted Lua and the
 *     artifact hands the engine that. THIS suite holds that path, in both
 *     directions, and that is what makes the list able to empty: an entry
 *     leaves when `diverges_on` would be empty, and neither path can quietly
 *     keep a claim the other one's suite is not checking.
 *
 * ## The population is the shared table, not the divergence list
 *
 * This suite used to ask only the 23 entries the divergence list holds, because
 * the fixture was expanded FROM that list. Two things were invisible from the
 * green that produced:
 *
 *   1. The other 75 cases of `ecma262_semantics.json` were never asked through
 *      a lowered artifact at all. A path's divergences cannot be enumerated by
 *      a list built from a DIFFERENT path's failures, so build-time lowering
 *      could answer any of those 75 wrongly with nothing to say so.
 *   2. Deleting an entry deleted its question. A list can only ratchet while
 *      something asks what the list no longer claims.
 *
 * `tools/generate_lowered_ecma262_fixture.py` now expands the shared table in
 * full, and the divergence list is read HERE, as the expectation about which
 * cases lowering gets wrong. So the two halves of the gate no longer share a
 * source — the fixture asks the language, and the list answers for the engine.
 *
 * ## The control, and why the green means something
 *
 * The binary carries the same document generated TWICE: `ecma262_lowered` with
 * `--script-engine lua`, and `ecma262_source` the way C++ has always been
 * generated. The second is not a bonus case, it is the CONTROL. A harness
 * comparing nothing, or a fixture whose cases had quietly stopped being asked,
 * would report the lowered machine perfect — and would report the source
 * machine perfect too, where the divergence list says it cannot be. One
 * document, one comparison, two artifacts, and the only difference between
 * them is the codegen flag under test.
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
 * ## Why a `cond=` case is asked twice — once as a guard, once as a probe
 *
 * §scxml-5.9.1 makes a `cond` that raises evaluate to false, so a guard that
 * did not hold is two findings wearing one verdict: the expression is false, or
 * the engine would not evaluate it. A case expecting `false` would otherwise
 * pass on an expression that could not be parsed — which for a LOWERED artifact
 * is the whole failure this gate exists to catch.
 *
 * The separator is a PROBE: the case's own expression, assigned to a bare
 * location in an earlier state. §scxml-5.4 routes a bare location through
 * `evaluateExpression`, the same entry point a `cond=` reaches, and leaves the
 * location unchanged when the expression fails — so a sentinel surviving is the
 * refusal. `agrees` refuses to read a condition verdict at all unless the probe
 * says the expression evaluated, and the probe is shown to distinguish both
 * outcomes on the same run before its word is used.
 *
 * ⚠ The separator used to be a third guard, `!(source)`, with the unguarded
 * fallthrough read as "neither held, so the engine refused". That reading was
 * measured wrong on 2026-08-29: `cond="a"` with `var a = 0` answers false
 * correctly under the runtime rewriter, and `cond="!(a)"` answers false too,
 * because the rewriter hands Lua `not a` and Lua counts 0 as true. Three cases
 * were reported as engine refusals that had not happened. Asking a DERIVED
 * expression made one entry's divergence (`!a`, §12.5.9 — which the shared
 * table already asks as a case of its own) surface as three other entries'
 * refusals. The fixture now asks only what the author wrote.
 */

#include "scripting/ScriptEngineProvider.h"
#include "scripting/ScriptSource.h"
#include <cstdint>
#include <fstream>
#include <gtest/gtest.h>
#include <iostream>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "ecma262_default_sm.h"
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

/// What a `cond=` case records: the guard held, or it did not.
///
/// ⚠ A third value used to sit here, reached by asking `!(source)` as well and
/// read as "neither held, so the engine refused the expression". Measured
/// 2026-08-29, that reading was WRONG on three cases at once: `cond="a"` with
/// `var a = 0` answers false correctly under the runtime rewriter, and
/// `cond="!(a)"` answers false too, because the rewriter hands Lua `not a` and
/// Lua counts 0 as true. The harness reported an engine refusal that had not
/// happened. A protocol that asks a DERIVED expression makes one entry's
/// divergence (`!a`, §12.5.9 — a case the shared table already asks on its own)
/// surface as three other entries' refusals.
///
/// So the fixture asks only the author's expression, and "could the engine
/// evaluate this at all" is the PROBE's question — see `Answer::evaluable`,
/// which `agrees` requires before it will read a condition verdict.
constexpr int64_t COND_HELD = 1;
constexpr int64_t COND_NOT_HELD = 2;

/// The sentinel the fixture parks in a slot before asking the engine, and the
/// value that slot still holds when the engine would not evaluate. §scxml-5.4
/// leaves an assignment's location unchanged when the expression fails and
/// §scxml-4.9 stops the block there, so the sentinel surviving IS the refusal.
/// Must match `PROBE_UNEVALUATED` in tools/generate_lowered_ecma262_fixture.py.
constexpr const char *UNEVALUATED = "<unevaluated>";

/// The two slots the fixture's probe controls record into.
///
/// Must match `CONTROL_REFUSED_VAR` / `CONTROL_EVALUABLE_VAR` there. They carry
/// the probe's discriminating power as DECLARED effects: one expression that
/// cannot be evaluated (a member of an absent object) and one that cannot be
/// refused (a literal).
constexpr const char *CONTROL_REFUSED = "ctlRefused";
constexpr const char *CONTROL_EVALUABLE = "ctlEvaluable";

/// The two paths from `datamodel="ecmascript"` into a Lua engine. This is the
/// `diverges_on` vocabulary, spelled here because this suite IS the second
/// path's contract; `ecma262_scoreboard_contract.rs` holds the same two names
/// against what `sce-build` derives, so a third path cannot appear in the list
/// without a lane that measures it.
constexpr const char *PATH_RUNTIME_REWRITER = "runtime-rewriter";
constexpr const char *PATH_BUILD_TIME_LOWERING = "build-time-lowering";

/// The floor. A table that shrank to nothing would score every engine
/// perfectly, which is the reason the two tables that feed this one carry the
/// same kind of floor. Same number as `MIN_CASES` in the fixture generator and
/// in `ecma262_scoreboard_contract.rs`.
constexpr size_t MIN_CASES = 55;

/// One case of the shared table, joined with whatever the divergence list says
/// about it.
struct Case {
    /// Index into the shared table's `cases`, which is the fixture's own key:
    /// state `dN` asks case N. The fixture is expanded from this file at
    /// configure time and read back here at run time, so the alignment is by
    /// position in ONE file rather than by a name either side could spell.
    size_t index = 0;
    /// The ECMAScript the fixture asks.
    std::string source;
    /// The ECMA-262 clause the table cites.
    std::string clause;
    /// True when the table asks it as a guard rather than as an `expr`.
    bool asCondition = false;
    /// The table's expectation, as JSON, so the comparison is by value and by
    /// type in one step.
    json expect;

    /// Does the divergence list carry an entry for this case at all?
    bool declared = false;
    /// `diverges_on` from that entry.
    std::set<std::string> paths;
    /// The entry's `needs`, carried so a failure names the family.
    std::string needs;

    bool divergesOnLowering() const {
        return paths.count(PATH_BUILD_TIME_LOWERING) != 0;
    }

    bool divergesOnRewriter() const {
        return paths.count(PATH_RUNTIME_REWRITER) != 0;
    }

    std::string name() const {
        return "[" + source + "] (" + clause + ")";
    }
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
 * @brief The shared table, each case carrying what the divergence list claims.
 *
 * `(source, clause)` identifies a case — `source` alone does not, since the
 * table asks `a && b` under two clauses. The join is computed here from the two
 * committed files rather than imported from the generator, for the reason
 * `docs/SCE_LUA_TRANSLATION_SEAM.md` states: *"A gate whose two halves share a
 * source is not a gate."*
 */
std::vector<Case> joinPopulation() {
    const json table = readJsonFile(SCE_ECMA262_CASES_PATH, "the shared ECMA-262 table");
    const json divergences = readJsonFile(SCE_LUA_DIVERGENCES_PATH, "the divergence list");

    // The paths this list says its entries may name. Read rather than assumed:
    // an entry naming a path nothing measures is a claim no lane can fault, so
    // it is refused here as well as in the Rust contract.
    std::set<std::string> declarable;
    if (divergences.contains("paths")) {
        for (const auto &p : divergences.at("paths")) {
            declarable.insert(p.get<std::string>());
        }
    }
    EXPECT_TRUE(declarable.count(PATH_BUILD_TIME_LOWERING) != 0)
        << SCE_LUA_DIVERGENCES_PATH << " does not list `" << PATH_BUILD_TIME_LOWERING
        << "` among the paths its entries may name, yet this suite is that path's contract. "
        << "Either the list stopped tracking the path this gate measures, or this gate is "
        << "measuring a backend the list is not about.";

    std::map<std::pair<std::string, std::string>, const json *> declaredBy;
    if (divergences.contains("divergences")) {
        const auto &list = divergences.at("divergences");
        for (size_t n = 0; n < list.size(); ++n) {
            const json &entry = list.at(n);
            auto key = std::make_pair(entry.value("source", std::string{}), entry.value("clause", std::string{}));
            if (!declaredBy.emplace(key, &entry).second) {
                ADD_FAILURE() << "divergence " << n << " [" << key.first << "] (" << key.second
                              << ") is listed twice; one of the two was never looked at";
            }
        }
    }

    std::vector<Case> population;
    std::set<std::pair<std::string, std::string>> matched;
    if (!table.contains("cases")) {
        return population;
    }
    const auto &cases = table.at("cases");
    for (size_t n = 0; n < cases.size(); ++n) {
        const json &entry = cases.at(n);
        Case c;
        c.index = n;
        c.source = entry.value("source", std::string{});
        c.clause = entry.value("clause", std::string{});
        c.asCondition = entry.value("form", std::string{}) == "condition";
        c.expect = entry.value("expect", json::object());

        auto hit = declaredBy.find(std::make_pair(c.source, c.clause));
        if (hit != declaredBy.end()) {
            matched.insert(hit->first);
            c.declared = true;
            c.needs = hit->second->value("needs", std::string{});
            if (!hit->second->contains("diverges_on")) {
                // Rule of this file's own header: an entry that does not say
                // which path it is about is not classified, and unclassified is
                // RED rather than a default. Defaulting it to the runtime path
                // would make every future entry silently exempt from THIS
                // suite, which is the escape hatch defeating its own gate.
                ADD_FAILURE() << "divergence " << c.name() << " carries no `diverges_on`. "
                              << "Two code paths reach the same Lua engine and this suite is one of them; "
                              << "an entry that does not say which path it is about cannot be checked by "
                              << "either, and a default would make it exempt from both.";
            } else {
                for (const auto &p : hit->second->at("diverges_on")) {
                    const auto path = p.get<std::string>();
                    if (declarable.count(path) == 0) {
                        ADD_FAILURE() << "divergence " << c.name() << " names the path `" << path << "`, which "
                                      << SCE_LUA_DIVERGENCES_PATH
                                      << " does not list under `paths`. A path no lane measures is a claim "
                                      << "nothing can fault.";
                    }
                    c.paths.insert(path);
                }
                if (c.paths.empty()) {
                    ADD_FAILURE() << "divergence " << c.name()
                                  << " has an EMPTY `diverges_on`. Every path answers it, so it is not a "
                                  << "divergence any more — delete the entry.";
                }
            }
        }
        population.push_back(std::move(c));
    }

    // A declared entry naming no case: the scoreboard gate refuses it too
    // (`sce-build/tests/ecma262_scoreboard_contract.rs`), and refusing it here
    // keeps this lane from silently dropping an expectation when that one is
    // not selected.
    for (const auto &pair : declaredBy) {
        if (matched.count(pair.first) == 0) {
            ADD_FAILURE() << "divergence [" << pair.first.first << "] (" << pair.first.second << ") names no case in "
                          << SCE_ECMA262_CASES_PATH
                          << ", so nothing in this suite asks it and its claim is unfalsifiable";
        }
    }
    return population;
}

/// The population, joined once. Read by every direction of the gate, so a
/// disagreement between them cannot come from having read the tables twice.
const std::vector<Case> &population() {
    static const std::vector<Case> joined = joinPopulation();
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

/// The four readings one answer slot can carry, which are four different
/// findings. See `emit_value_case` in the fixture generator: the ORDER of the
/// assignments is what keeps them from collapsing into one absence.
enum class Reading {
    /// `answers.rN` absent — the case's state was never entered. Never a pass,
    /// and not the same finding as a wrong answer: it says the fixture and this
    /// harness disagree about the population, or the machine stopped early.
    NotReached,
    /// The slot still holds the sentinel — the setup or the expression was
    /// refused by the engine.
    Refused,
    /// The slot is absent although the case was reached — the expression
    /// evaluated, to null/undefined. This is what the shared table spells
    /// `{"empty": true}`, and it is the reading that used to be impossible.
    Empty,
    /// The slot holds a value.
    Value,
};

struct Answer {
    Reading reading = Reading::NotReached;
    json value;
    /// Could the engine evaluate the case's expression at all?
    ///
    /// For a condition case this is the PROBE's answer, and it is load-bearing
    /// rather than diagnostic: §scxml-5.9.1 makes a guard the engine refused
    /// evaluate to false, so without it every case whose ECMA-262 answer is
    /// false would pass on an expression that could not be parsed. `agrees`
    /// refuses to read a condition verdict when this is false.
    ///
    /// For a value case the reading already answers it — `Refused` IS the
    /// refusal.
    bool evaluable = false;

    bool isValue() const {
        return reading == Reading::Value;
    }
};

/// Did the probe say the engine could evaluate this condition case?
///
/// Read in the state AFTER the one that wrote it — §scxml-4.9 stops a block at
/// the element that raised, so a probe read in its own block would go down
/// with the refusal it exists to report.
bool probeSaysEvaluable(const json &answers, const Case &c) {
    const auto probe = "v" + std::to_string(c.index);
    if (!answers.contains(probe)) {
        // The probe assignment yielded null/undefined, which the engine's JSON
        // encoding omits. It evaluated; it just produced nothing to hold.
        return answers.contains("r" + std::to_string(c.index));
    }
    const json &recorded = answers.at(probe);
    return !(recorded.is_string() && recorded.get<std::string>() == UNEVALUATED);
}

Answer readAnswer(const json &answers, const Case &c) {
    Answer answer;
    if (!answers.is_object()) {
        return answer;
    }
    if (!answers.contains("r" + std::to_string(c.index))) {
        return answer;
    }
    const auto slot = "d" + std::to_string(c.index);
    if (!answers.contains(slot)) {
        answer.reading = Reading::Empty;
        answer.evaluable = !c.asCondition;
        return answer;
    }
    const json &recorded = answers.at(slot);
    if (recorded.is_string() && recorded.get<std::string>() == UNEVALUATED) {
        answer.reading = Reading::Refused;
        return answer;
    }
    answer.reading = Reading::Value;
    answer.value = recorded;
    answer.evaluable = c.asCondition ? probeSaysEvaluable(answers, c) : true;
    return answer;
}

/// Does one recorded answer agree with ECMA-262?
///
/// Numbers are compared as doubles because the two engine families hold them
/// differently and the shared table says so in as many words ("an engine may
/// hold it as an integer or a double"). Everything else is compared by type as
/// well as by value: `0` and `false` and `""` are three different answers, and
/// the truthiness family is exactly where a Lua-shaped rewriter confuses them.
bool agrees(const Answer &answer, const Case &c) {
    if (c.asCondition) {
        if (!answer.isValue() || !answer.value.is_number()) {
            return false;
        }
        // §scxml-5.9.1 makes a `cond` the engine refused evaluate to FALSE, so
        // a guard that did not hold is two findings wearing one verdict. The
        // probe is what separates them, and a refusal is never an answer about
        // the language — however well it happens to match the expectation.
        if (!answer.evaluable) {
            return false;
        }
        if (!c.expect.contains("bool")) {
            return false;
        }
        const bool wanted = c.expect.at("bool").get<bool>();
        return answer.value.get<int64_t>() == (wanted ? COND_HELD : COND_NOT_HELD);
    }
    if (c.expect.contains("empty")) {
        // The one shape that is an ABSENCE. It is a pass only when the case was
        // reached and the slot went from the sentinel to nothing, which is the
        // engine having evaluated the expression to null/undefined.
        return answer.reading == Reading::Empty;
    }
    if (!answer.isValue()) {
        return false;
    }
    if (c.expect.contains("bool")) {
        return answer.value.is_boolean() && answer.value.get<bool>() == c.expect.at("bool").get<bool>();
    }
    if (c.expect.contains("number")) {
        return answer.value.is_number() && answer.value.get<double>() == c.expect.at("number").get<double>();
    }
    if (c.expect.contains("string")) {
        return answer.value.is_string() && answer.value.get<std::string>() == c.expect.at("string").get<std::string>();
    }
    return false;
}

/// What one probe control recorded, spelled the one way the census prints it
/// and the assertions below read it.
///
/// A slot the run never wrote reads as `<absent>` rather than as an absence:
/// both assertions below are about a VALUE, and a missing field that quietly
/// satisfied either of them would be the hole they exist to close.
std::string controlReading(const json &answers, const char *slot) {
    if (!answers.is_object() || !answers.contains(slot)) {
        return "<absent>";
    }
    const json &value = answers.at(slot);
    return value.is_string() ? value.get<std::string>() : value.dump();
}

/// The refusal probe, shown to report BOTH outcomes on this artifact's run.
///
/// `agrees` refuses to read a condition verdict whose probe says the engine
/// could not evaluate the expression, so a probe stuck on one answer decides a
/// whole test by itself: stuck on "refused" makes every condition case a
/// divergence, stuck on "evaluated" makes a genuine refusal read as the answer
/// `false`.
///
/// ⚠ This used to be a COUNT over the shared table — at least one refusal and
/// at least one evaluation among the table's own condition cases. On
/// 2026-08-29 the refusal count reached ZERO, because the engine's lowering
/// seam had grown to answer every guard in the table, and the control failed
/// on a repair. A control whose zero is forbidden forbids the finish line, and
/// the population it was counting is not the thing it is about. The fixture
/// now opens with two states that produce the outcomes on PURPOSE — a member
/// of an absent object, which raises in ECMA-262 and in Lua alike, and a
/// literal, which nothing can refuse — so what is asked here is a declared
/// effect rather than an accident of the corpus.
void assertProbeDistinguishes(const json &answers, const std::string &label) {
    ASSERT_TRUE(answers.is_object()) << label << ": the run recorded no answer object at all";

    const auto reading = [&answers](const char *slot) { return controlReading(answers, slot); };

    EXPECT_EQ(reading(CONTROL_REFUSED), std::string(UNEVALUATED))
        << label
        << ": the fixture probes `answers.missing.deep`, which cannot be evaluated — reading a "
           "member of an absent object raises in ECMA-262 and in Lua alike — and the probe "
           "reported `"
        << reading(CONTROL_REFUSED)
        << "`. The probe is therefore not reporting §scxml-5.9.1 refusals at all, and every guard the "
           "engine would not parse reads below as the answer `false`.";

    EXPECT_NE(reading(CONTROL_EVALUABLE), std::string(UNEVALUATED))
        << label
        << ": the fixture probes the literal `1`, which nothing can refuse, and the probe reported it "
           "as unevaluated. The probe is therefore stuck on refusal, and every condition case below is "
           "a divergence by construction.";
}

/// The condition verdicts are a population, and an empty one measures nothing.
///
/// Separate from [`assertProbeDistinguishes`] because it is a different
/// question: that one asks whether the probe can tell the two apart, this one
/// asks whether any condition case survived it to be judged. Its zero is
/// reachable only by the engine refusing every guard in the table, which is a
/// finding rather than a finish line.
void assertConditionVerdictsExist(const json &answers, const std::vector<Case> &cases, const std::string &label) {
    size_t evaluable = 0;
    for (const auto &c : cases) {
        if (c.asCondition && probeSaysEvaluable(answers, c)) {
            ++evaluable;
        }
    }
    EXPECT_GT(evaluable, 0u) << label
                             << ": the engine refused every condition case in the shared table, so no "
                                "condition verdict below was read and that half of this test measured nothing";
}

std::string describe(const Answer &answer, const Case &c) {
    switch (answer.reading) {
    case Reading::NotReached:
        return "<the case was never reached>";
    case Reading::Refused:
        return "<the engine refused the expression>";
    case Reading::Empty:
        return "<undefined — the slot was evaluated away>";
    case Reading::Value:
        break;
    }
    std::string shown = answer.value.dump();
    if (c.asCondition && answer.value.is_number()) {
        switch (answer.value.get<int64_t>()) {
        case COND_HELD:
            shown += " (the guard held)";
            break;
        case COND_NOT_HELD:
            shown += " (the guard did not hold)";
            break;
        default:
            break;
        }
        shown += answer.evaluable ? ", and the engine evaluated the expression"
                                  : ", and the engine REFUSED the expression — so §scxml-5.9.1's false is "
                                    "what the site saw, not an answer about the language";
    }
    return shown;
}

std::string joined(const std::vector<std::string> &lines) {
    std::string text;
    for (const auto &line : lines) {
        text += "  " + line + "\n";
    }
    return text;
}

TEST(LoweredEcma262, ThePopulationIsTheWholeSharedTable) {
    const auto &cases = population();
    EXPECT_GE(cases.size(), MIN_CASES)
        << "only " << cases.size() << " case(s) read from " << SCE_ECMA262_CASES_PATH
        << ". This suite asks the LANGUAGE and uses " << SCE_LUA_DIVERGENCES_PATH
        << " only as the expectation about which of those answers lowering gets wrong, so a short "
        << "table is a smaller suite reported under the same name.";
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

/// Every case must have been ASKED before any verdict about it is worth having.
///
/// Split from the measurement below on purpose. A stale fixture, a machine that
/// stopped early, or a generator and this harness disagreeing about the
/// population all arrive as an absence — and an absence read as a wrong answer
/// sends the reader to the frontend for a defect that is in the plumbing.
TEST(LoweredEcma262, EveryCaseWasActuallyAsked) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json lowered = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");
    const json source = runMachine<::SCE::Generated::ecma262_source::ecma262_source>("the source-passing artifact");

    for (const auto &pair : {std::make_pair("lowered", &lowered), std::make_pair("source", &source)}) {
        std::vector<std::string> unasked;
        for (const auto &c : cases) {
            if (readAnswer(*pair.second, c).reading == Reading::NotReached) {
                unasked.push_back("d" + std::to_string(c.index) + " " + c.name());
            }
        }
        EXPECT_TRUE(unasked.empty())
            << unasked.size() << " case(s) of " << cases.size() << " were never reached in the " << pair.first
            << " artifact. The fixture marks each case as it enters it, so this is the fixture and this harness\n"
            << "disagreeing about the population — a stale generated document, or a machine that stopped early.\n"
            << joined(unasked);
    }

    // The census, printed on every run rather than only on a red one. A number
    // that exists only in a failure message is a number nobody can cite from a
    // green build, which is how the counts in this axis's documents came to
    // outlive the tables they were taken from — twice, in as many words, in
    // `lua_engine_divergences.json`'s own header. The gate lifts this line out
    // of ctest's verbose log and re-prints it, so a passing run states what it
    // asked.
    size_t loweredWrong = 0;
    size_t sourceWrong = 0;
    size_t declaredLowering = 0;
    size_t declaredRewriter = 0;
    for (const auto &c : cases) {
        if (!agrees(readAnswer(lowered, c), c)) {
            ++loweredWrong;
        }
        if (!agrees(readAnswer(source, c), c)) {
            ++sourceWrong;
        }
        declaredLowering += c.divergesOnLowering() ? 1 : 0;
        declaredRewriter += c.divergesOnRewriter() ? 1 : 0;
    }
    // The probe controls are printed here for the same reason the counts are,
    // and for one more. `assertProbeDistinguishes` reads them in the two tests
    // below, where a green run says nothing about them at all — so deleting
    // that call would leave every condition verdict resting on a probe nobody
    // asks about, and every suite in this file would still pass. Printed here
    // they are a claim the GATE can hold: the census must carry both readings,
    // for both artifacts, with the refusal side still refusing.
    std::cout << "LoweredEcma262 census: population=" << cases.size() << " lowered-wrong=" << loweredWrong
              << " source-wrong=" << sourceWrong << " declared-" << PATH_BUILD_TIME_LOWERING << "=" << declaredLowering
              << " declared-" << PATH_RUNTIME_REWRITER << "=" << declaredRewriter
              << " lowered-control-refused=" << controlReading(lowered, CONTROL_REFUSED)
              << " lowered-control-evaluable=" << controlReading(lowered, CONTROL_EVALUABLE)
              << " source-control-refused=" << controlReading(source, CONTROL_REFUSED)
              << " source-control-evaluable=" << controlReading(source, CONTROL_EVALUABLE) << std::endl;
}

/**
 * @brief The ratchet: lowering diverges EXACTLY where it is declared to.
 *
 * Both directions, because a one-sided verdict rots — the lesson the runtime
 * path's own suite records: *"Asking only 'nothing new disagrees' lets the
 * declared list keep claiming a divergence that has since been repaired, and
 * asking only 'the declared ones still disagree' lets a new one arrive
 * unremarked, which is how the count reached 44 while a comment said 26."*
 *
 * So:
 *
 *   - a case lowering answers wrongly WITHOUT `build-time-lowering` in its
 *     `diverges_on` is red, and closing it means fixing the frontend or, if the
 *     divergence is real, marking the path;
 *   - a case marked `build-time-lowering` that lowering answers CORRECTLY is
 *     red, and closing it means removing that path from `diverges_on` — and
 *     deleting the entry outright when nothing is left in it.
 *
 * The second direction is what lets `lua_engine_divergences.json` empty. Before
 * it, an entry could only leave the file when the runtime rewriter was
 * repaired, and the plan of record is to RETIRE that rewriter for generated
 * code rather than repair it — so the file had no reachable end state at all.
 */
TEST(LoweredEcma262, TheLoweredArtifactDivergesExactlyWhereItIsDeclaredTo) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json answers = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");

    // The probe decides which condition verdicts are read here too, and it is a
    // DIFFERENT run of it: this artifact was lowered at build time, so a probe
    // that stopped distinguishing on this side would be invisible to the
    // control on the source-passing side.
    assertProbeDistinguishes(answers, "the lowered artifact");
    assertConditionVerdictsExist(answers, cases, "the lowered artifact");

    std::vector<std::string> undeclared;
    std::vector<std::string> repaired;
    for (const auto &c : cases) {
        const Answer answer = readAnswer(answers, c);
        const bool ok = agrees(answer, c);

        if (!ok && !c.divergesOnLowering()) {
            undeclared.push_back(c.name() + " — answered " + describe(answer, c) + ", ECMA-262 says " +
                                 c.expect.dump() +
                                 (c.declared ? "  [declared, but only on: " +
                                                   [&c] {
                                                       std::string paths;
                                                       for (const auto &p : c.paths) {
                                                           paths += (paths.empty() ? "" : ", ") + p;
                                                       }
                                                       return paths;
                                                   }() +
                                                   "]"
                                             : "  [not in the divergence list at all]"));
        }
        if (ok && c.divergesOnLowering()) {
            repaired.push_back(c.name() + " — answered " + describe(answer, c) +
                               ", which IS what ECMA-262 says (needs: " + c.needs + ")");
        }
    }

    EXPECT_TRUE(undeclared.empty())
        << undeclared.size() << " case(s) the LOWERED artifact answers differently from ECMA-262 without\n"
        << SCE_LUA_DIVERGENCES_PATH << " declaring `" << PATH_BUILD_TIME_LOWERING << "` for them.\n"
        << "This is build-time lowering, so the runtime rewriter was never reached: the frontend emitted Lua\n"
        << "that does not mean what the author wrote. Fix `sce-build`'s ECMAScript frontend, or — if the\n"
        << "divergence is real and staying — add `" << PATH_BUILD_TIME_LOWERING << "` to that entry's\n"
        << "`diverges_on` so the claim is written down where the next reader will find it.\n"
        << joined(undeclared);

    EXPECT_TRUE(repaired.empty())
        << repaired.size() << " case(s) are declared to diverge on `" << PATH_BUILD_TIME_LOWERING
        << "` and the lowered artifact answers them CORRECTLY.\n"
        << "Remove that path from the entry's `diverges_on` in " << SCE_LUA_DIVERGENCES_PATH
        << " — and if `diverges_on` is then empty, delete the entry: every path answers it, so it is not a\n"
        << "divergence any more. A list that keeps a repaired claim understates the engine as surely as a\n"
        << "missing entry overstates it, and this direction is the one that lets the list reach zero.\n"
        << joined(repaired);
}

/**
 * @brief Lowering may fix answers; it may not lose one.
 *
 * The cross-artifact claim, and the only one that needs both machines at once.
 * It is separate from either path's contract because it is not about a
 * declaration: a case the un-lowered artifact answers correctly and the lowered
 * one does not is a regression in the frontend, whatever any list says.
 */
TEST(LoweredEcma262, LoweringLosesNoAnswerTheRuntimeRewriterAlreadyHad) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json lowered = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");
    const json source = runMachine<::SCE::Generated::ecma262_source::ecma262_source>("the source-passing artifact");

    std::vector<std::string> lost;
    for (const auto &c : cases) {
        const Answer fromSource = readAnswer(source, c);
        if (!agrees(fromSource, c)) {
            continue;
        }
        const Answer fromLowered = readAnswer(lowered, c);
        if (!agrees(fromLowered, c)) {
            lost.push_back(c.name() + " — without lowering: " + describe(fromSource, c) +
                           "; with lowering: " + describe(fromLowered, c) + "; ECMA-262 says " + c.expect.dump());
        }
    }

    EXPECT_TRUE(lost.empty()) << lost.size()
                              << " case(s) are answered correctly WITHOUT lowering and wrongly WITH it.\n"
                                 "Build-time lowering lost an answer the runtime rewriter already had, which is a\n"
                                 "regression in the frontend and not a divergence to declare.\n"
                              << joined(lost);
}

/**
 * @brief A tree that selected Lua emits Lua, without being asked twice.
 *
 * `-DSCE_SCRIPT_ENGINE=lua` says this tree's C++ evaluates its datamodel on
 * Lua. An artifact built here can therefore only run on a Lua engine, so the
 * language it hands that engine should be Lua whether or not the CMake caller
 * says so — and `sce_add_state_machine` now derives exactly that.
 *
 * A derivation is a claim about what a build PRODUCES, so it is asked of the
 * product. `ecma262_default` is generated by the one call in the tree that
 * names no `SCRIPT_ENGINE_LANGUAGE`, and it must be indistinguishable from the
 * explicitly-lowered artifact: same answers, case for case. The gate counts its
 * `ScriptSource::lua(...)` pairs from the outside as well, so "it was lowered"
 * and "it answers like the lowered one" are two independent readings.
 *
 * ⚠ This is the step that removed generated C++ as a consumer of
 * `EcmaScriptToLuaTransformer`. It did NOT empty
 * `tests/ecmascript/lua_engine_divergences.json` — measured 2026-08-29, both
 * suites holding that list reach the engine by routes no codegen default can
 * touch, so the list emptied when the frontend answered all 98 and the
 * rewriter was retired separately. What this step bought was retiring it
 * without taking generated code down with it.
 */
TEST(LoweredEcma262, ATreeThatSelectedLuaEmitsLuaWithoutBeingAsked) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json lowered = runMachine<::SCE::Generated::ecma262_lowered::ecma262_lowered>("the lowered artifact");
    const json derived = runMachine<::SCE::Generated::ecma262_default::ecma262_default>("the derived-default artifact");

    std::vector<std::string> differed;
    for (const auto &c : cases) {
        const Answer fromLowered = readAnswer(lowered, c);
        const Answer fromDerived = readAnswer(derived, c);
        if (agrees(fromLowered, c) == agrees(fromDerived, c)) {
            continue;
        }
        differed.push_back(c.name() + " — asked for lua: " + describe(fromLowered, c) +
                           "; asked for nothing: " + describe(fromDerived, c) + "; ECMA-262 says " + c.expect.dump());
    }

    EXPECT_TRUE(differed.empty())
        << differed.size()
        << " case(s) are answered differently by an artifact that ASKED for --script-engine lua and one that\n"
           "asked for nothing in the same -DSCE_SCRIPT_ENGINE=lua tree. The derived default is what makes those\n"
           "two the same build, so a difference means `sce_add_state_machine` did not derive it — the tree\n"
           "selected a Lua engine and then generated a machine that hands it ECMAScript.\n"
        << joined(differed);

    // The count is the other half, and it is deliberately not a comparison
    // against the subject: two artifacts can agree by both being wrong. This
    // one asks the ARTIFACT what language it was emitted for.
    size_t loweredCorrect = 0;
    for (const auto &c : cases) {
        loweredCorrect += agrees(readAnswer(derived, c), c) ? 1 : 0;
    }
    EXPECT_EQ(loweredCorrect, cases.size())
        << "the derived-default artifact answers " << loweredCorrect << " of " << cases.size()
        << " cases as ECMA-262 does. In this tree it is a lowered artifact, so it should answer all of them —\n"
           "the same claim `TheLoweredArtifactDivergesExactlyWhereItIsDeclaredTo` makes about the explicit one.";
}

/**
 * @brief The control is a ratchet too: it diverges EXACTLY where declared.
 *
 * The un-lowered artifact is what C++ emits today — generated code handing the
 * author's ECMAScript to whichever engine the tree selected — so this is the
 * `runtime-rewriter` path reached the way a DOCUMENT reaches it.
 * `ecmascript_semantics_test` holds the same path reached by a direct evaluate,
 * and the two are different routes to one engine; holding both is what keeps a
 * divergence that only shows up through generated code from having no home.
 *
 * It replaced a floor (`MIN_SOURCE_DIVERGENCES`, "at least 22 of the declared
 * entries must still come back") plus an attribution test for the entries the
 * floor let through. That pair had a hole exactly the shape of its allowance: a
 * case the control got wrong while naming NO entry was counted by neither, and
 * the census reported a number nobody compared. Measured 2026-08-29 on the
 * first census — 25 wrong against 23 declared, and the three extra were being
 * tolerated by nothing more than the absence of a predicate.
 *
 * An equality needs no allowance, so there is nothing left to hide in. It also
 * makes this direction able to SHRINK the list: a rewriter divergence that gets
 * repaired turns this red asking for the entry to drop `runtime-rewriter`,
 * which is the same closing move the lowered side has.
 */
TEST(LoweredEcma262, TheSourcePassingArtifactDivergesExactlyWhereItIsDeclaredTo) {
    const auto &cases = population();
    ASSERT_FALSE(cases.empty());

    const json answers = runMachine<::SCE::Generated::ecma262_source::ecma262_source>("the source-passing artifact");

    // The probe's own control, and it has to hold BEFORE any verdict below is
    // used: `agrees` refuses a condition case whose probe says the engine could
    // not evaluate it, so a probe stuck on one answer would decide this whole
    // test by itself.
    assertProbeDistinguishes(answers, "the source-passing artifact");
    assertConditionVerdictsExist(answers, cases, "the source-passing artifact");

    std::vector<std::string> undeclared;
    std::vector<std::string> repaired;
    for (const auto &c : cases) {
        const Answer answer = readAnswer(answers, c);
        const bool ok = agrees(answer, c);
        if (!ok && !c.divergesOnRewriter()) {
            undeclared.push_back(c.name() + " — answered " + describe(answer, c) + ", ECMA-262 says " +
                                 c.expect.dump() + (c.declared ? "  [declared, but not on this path]" : ""));
        }
        if (ok && c.divergesOnRewriter()) {
            repaired.push_back(c.name() + " — answered " + describe(answer, c) +
                               ", which IS what ECMA-262 says (needs: " + c.needs + ")");
        }
    }

    EXPECT_TRUE(undeclared.empty())
        << undeclared.size() << " case(s) the SOURCE-passing artifact answers differently from ECMA-262 without\n"
        << SCE_LUA_DIVERGENCES_PATH << " declaring `" << PATH_RUNTIME_REWRITER << "` for them.\n"
        << "That artifact is what C++ emits today, so each of these is a divergence a consumer already has\n"
        << "and nothing has written down. Declare it, or fix it — but a wrong answer that no list claims is\n"
        << "the one thing this pair of files exists to make impossible.\n"
        << joined(undeclared);

    EXPECT_TRUE(repaired.empty())
        << repaired.size() << " case(s) are declared to diverge on `" << PATH_RUNTIME_REWRITER
        << "` and the source-passing artifact answers them CORRECTLY.\n"
        << "The rewriter gained those cases. Drop that path from the entry in " << SCE_LUA_DIVERGENCES_PATH
        << " — and if `diverges_on` is then empty, delete the entry. A list that keeps a repaired claim\n"
        << "understates the engine as surely as a missing entry overstates it, and this is the direction that\n"
        << "lets the runtime column reach zero.\n"
        << joined(repaired);
}

}  // namespace
