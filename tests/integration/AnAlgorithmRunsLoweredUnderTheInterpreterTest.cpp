// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Interpreter as a seventh engine over the numerical conformance cases (E11).
//
// The six generated backends are held to `tests/forge/conformance/
// numerical_reference.json`, whose cases are written by an independent model
// (`gen_cases.py`). An `algorithm` document also runs under the Interpreter once
// `sce-codegen lower-algorithm` has turned it into ECMAScript over the runtime
// library (`SceStatic`, docs/SCE_ACCEPTED_SUBSET.md §2.15), and that lowering is
// new code that re-implements the integer contract (SCE_FORGE.md §3.4.1) in a
// third language. This runs the same cases through it, in the script engine the
// Interpreter ships with, so a disagreement is a defect in the lowering or in
// the library and not a difference in what each engine was asked.
//
// Nothing is listed. The fixtures are the algorithm entries of the catalog the
// backends read, the document of each is what the catalog names, and whether
// the Interpreter can run one is asked of `sce-codegen`: an algorithm with a
// construct that has no lowering yet is refused with a diagnostic naming it, and
// starts being compared the day the lowering covers it.
//
// What an integer is here is the one place the Interpreter cannot match a
// backend. A Number holds an integer exactly only up to 2^53 - 1, so a case with
// an argument past it is not asked, and an operation whose operand or result is
// past it fails `unrepresentable` — a name no backend has, raised where the
// answer would have been a value the engine cannot hold. Both are counted and
// printed, and neither is a disagreement. A failure that names another contract
// name than the case's, or a value that differs, is.

#include "SCXMLTypes.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <variant>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

const std::filesystem::path kRoot = std::filesystem::path(SCE_PROJECT_ROOT);
const std::filesystem::path kCatalog = kRoot / "tests/forge/conformance/fixtures.json";
const std::filesystem::path kReference = kRoot / "tests/forge/conformance/numerical_reference.json";
const std::filesystem::path kResources = kRoot / "tests/forge/resources";

/// The integers a Number holds exactly, and so the ones this engine is asked.
constexpr int64_t kSafe = 9007199254740991LL;

nlohmann::json readJson(const std::filesystem::path &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    nlohmann::json document;
    in >> document;
    return document;
}

/// What a command wrote to stdout, and how it ended.
struct Ran {
    int status = -1;
    std::string output;
};

Ran capture(const std::string &command) {
    Ran ran;
    FILE *pipe = popen(command.c_str(), "r");
    if (pipe == nullptr) {
        return ran;
    }
    char buffer[65536];
    while (std::fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        ran.output += buffer;
    }
    ran.status = pclose(pipe);
    return ran;
}

/// One algorithm fixture of the catalog: its document, and the reference cases
/// the backends are held to.
struct Fixture {
    std::string name;
    std::string document;
    std::string section;
};

/// The algorithm entries of the catalog, in its order. The document of one is
/// what the catalog names — a standard document — or, for a fixture the catalog
/// gives none, the resource of the same name. Whether it can be lowered is not
/// judged here: a record, list or bytes among its arguments or its result is
/// refused by `sce-codegen lower-algorithm`, and the refusal is what is read.
std::vector<Fixture> algorithmFixtures() {
    const nlohmann::json catalog = readJson(kCatalog);
    std::vector<Fixture> found;
    for (const auto &entry : catalog.at("fixtures")) {
        if (entry.value("kind", std::string{}) != "algorithm") {
            continue;
        }
        Fixture fixture;
        fixture.name = entry.at("name").get<std::string>();
        fixture.section = entry.value("ref_section", std::string{"pure_functions"});
        fixture.document = entry.contains("document") ? entry.at("document").get<std::string>()
                                                      : (kResources / (fixture.name + ".scxml")).string();
        found.push_back(std::move(fixture));
    }
    return found;
}

/// `sce-codegen lower-algorithm <document>`: the install expression and the name
/// it installs under when it succeeds; the diagnostic when it refuses.
struct Lowered {
    bool ok = false;
    std::string symbol;
    std::string install;
    std::string refusal;
};

Lowered lower(const std::string &document) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    EXPECT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    Lowered lowered;
    if (bin == nullptr) {
        return lowered;
    }
    const std::string base = std::string("\"") + bin + "\" lower-algorithm \"" + document + "\" --error-format=json";
    const Ran ran = capture(base + " 2>/dev/null");
    if (ran.status == 0) {
        const nlohmann::json line = nlohmann::json::parse(ran.output, nullptr, false);
        if (!line.is_object()) {
            lowered.refusal = "unreadable output: " + ran.output.substr(0, 200);
            return lowered;
        }
        lowered.ok = true;
        lowered.symbol = line.at("symbol").get<std::string>();
        lowered.install = line.at("install").get<std::string>();
        return lowered;
    }
    const Ran refused = capture(base + " 2>&1 >/dev/null");
    const auto eol = refused.output.find('\n');
    const nlohmann::json diagnostic = nlohmann::json::parse(refused.output.substr(0, eol), nullptr, false);
    lowered.refusal = diagnostic.is_object() ? diagnostic.value("code", std::string{"?"}) + ": " +
                                                   diagnostic.value("message", std::string{})
                                             : refused.output.substr(0, 200);
    return lowered;
}

/// An argument as script source: a list or bytes is an array, a record an
/// object, each spelled from what it holds. An integer a Number holds exactly is
/// a Number and one it does not is a BigInt literal, which is the one form the
/// lowered functions hold an integer in (`SceStatic`, docs/SCE_ACCEPTED_SUBSET.md
/// 2.15); nothing is left unspelled but a value of a kind the reference does not
/// carry.
bool spell(const nlohmann::json &value, std::string &out) {
    if (value.is_array()) {
        out = "[";
        for (size_t index = 0; index < value.size(); ++index) {
            std::string element;
            if (!spell(value[index], element)) {
                return false;
            }
            out += (index == 0 ? "" : ", ") + element;
        }
        out += "]";
        return true;
    }
    if (value.is_object()) {
        out = "{";
        bool first = true;
        for (const auto &member : value.items()) {
            std::string element;
            if (!spell(member.value(), element)) {
                return false;
            }
            out += (first ? "" : ", ") + nlohmann::json(member.key()).dump() + ": " + element;
            first = false;
        }
        out += "}";
        return true;
    }
    if (value.is_boolean()) {
        out = value.get<bool>() ? "true" : "false";
        return true;
    }
    if (value.is_number_integer()) {
        if (value.is_number_unsigned()) {
            const uint64_t number = value.get<uint64_t>();
            out = std::to_string(number) + (number > static_cast<uint64_t>(kSafe) ? "n" : "");
            return true;
        }
        const int64_t number = value.get<int64_t>();
        out = std::to_string(number) + (number > kSafe || number < -kSafe ? "n" : "");
        return true;
    }
    if (value.is_number_float()) {
        out = value.dump();
        return true;
    }
    return false;
}

/// What one call answered: a value, or a failure and the name it gave itself.
struct Answer {
    bool failed = false;
    std::string failure;  // sceFailure, empty when the throw named none
    std::string message;
    nlohmann::json value;
};

/// JSON has no BigInt, so the answer carries one as `{"sceBigInt": "<digits>"}`
/// and this puts the integer back.
void restoreBigInts(nlohmann::json &value) {
    if (value.is_object() && value.size() == 1 && value.contains("sceBigInt")) {
        const std::string digits = value.at("sceBigInt").get<std::string>();
        value = digits.front() == '-' ? nlohmann::json(std::stoll(digits)) : nlohmann::json(std::stoull(digits));
        return;
    }
    if (value.is_array() || value.is_object()) {
        for (auto &element : value) {
            restoreBigInts(element);
        }
    }
}

Answer call(::SCE::IScriptEngine &engine, const std::string &session, const std::string &symbol,
            const std::string &arguments) {
    const std::string source =
        "(function () { try { return JSON.stringify({ ok: SceStatic.algorithms." + symbol + "(" + arguments +
        ") }, function (key, value) { return typeof value === 'bigint' ? { sceBigInt: value.toString() } : value; "
        "}); } catch (e) { return JSON.stringify({ failure: (e && e.sceFailure) || null, message: String(e && "
        "e.message) }); } })()";
    Answer answer;
    auto result = engine.evaluateExpression(session, source).get();
    if (!result.isSuccess()) {
        answer.failed = true;
        answer.message = "the engine did not evaluate the call: " + result.getErrorMessage();
        return answer;
    }
    const ::ScriptValue &internal = result.getInternalValue();
    if (!std::holds_alternative<std::string>(internal)) {
        answer.failed = true;
        answer.message = "the call answered something other than the JSON text asked for";
        return answer;
    }
    const nlohmann::json parsed = nlohmann::json::parse(std::get<std::string>(internal), nullptr, false);
    if (!parsed.is_object()) {
        answer.failed = true;
        answer.message = "unreadable: " + std::get<std::string>(internal);
        return answer;
    }
    if (parsed.contains("ok")) {
        answer.value = parsed.at("ok");
        restoreBigInts(answer.value);
        return answer;
    }
    answer.failed = true;
    answer.failure = parsed.at("failure").is_string() ? parsed.at("failure").get<std::string>() : std::string{};
    answer.message = parsed.value("message", std::string{});
    return answer;
}

/// Whether two values are the same: a list or bytes by its elements in order, a
/// record by its fields whatever order they were written in, and the rest as a
/// number or a truth value is.
bool sameValue(const nlohmann::json &got, const nlohmann::json &want) {
    if (got.is_array() || want.is_array()) {
        if (!got.is_array() || !want.is_array() || got.size() != want.size()) {
            return false;
        }
        for (size_t index = 0; index < got.size(); ++index) {
            if (!sameValue(got[index], want[index])) {
                return false;
            }
        }
        return true;
    }
    if (got.is_object() || want.is_object()) {
        if (!got.is_object() || !want.is_object() || got.size() != want.size()) {
            return false;
        }
        for (const auto &member : want.items()) {
            if (!got.contains(member.key()) || !sameValue(got.at(member.key()), member.value())) {
                return false;
            }
        }
        return true;
    }
    if (got.is_boolean() || want.is_boolean()) {
        return got.is_boolean() && want.is_boolean() && got.get<bool>() == want.get<bool>();
    }
    // Two integers are compared as integers: past 2^53 a double is a different
    // value from its neighbour.
    if (got.is_number_integer() && want.is_number_integer()) {
        return got == want;
    }
    if (got.is_number() && want.is_number()) {
        const double a = got.get<double>();
        const double b = want.get<double>();
        return a == b || std::abs(a - b) <= 1e-9 * std::max(1.0, std::abs(b));
    }
    return false;
}

/// What a fixture's cases came to.
struct Tally {
    size_t cases = 0;
    size_t notAsked = 0;         // an argument beyond what a Number holds exactly
    size_t unrepresentable = 0;  // asked, and the engine could not hold an operand or a result
    size_t compared = 0;
    size_t agreed = 0;
    std::vector<std::string> disagreements;
};

std::string spelled(const nlohmann::json &args) {
    return args.dump();
}

Tally run(const Fixture &fixture, const Lowered &lowered, const nlohmann::json &cases) {
    Tally tally;
    auto &engine = ::SCE::ScriptEngineProvider::getScriptEngine();
    const std::string session = "algorithm_lowered_" + fixture.name;
    EXPECT_TRUE(engine.createSession(session, "")) << "no session for " << fixture.name;
    auto installed = engine.executeScript(session, lowered.install).get();
    EXPECT_TRUE(installed.isSuccess()) << fixture.name << ": the install did not run: " << installed.getErrorMessage();
    if (!installed.isSuccess()) {
        engine.destroySession(session);
        return tally;
    }
    for (size_t index = 0; index < cases.size(); ++index) {
        const auto &entry = cases[index];
        ++tally.cases;
        std::string arguments;
        bool asked = true;
        for (const auto &argument : entry.at("args")) {
            std::string one;
            if (!spell(argument, one)) {
                asked = false;
                break;
            }
            arguments += (arguments.empty() ? "" : ", ") + one;
        }
        if (!asked) {
            ++tally.notAsked;
            continue;
        }
        const Answer answer = call(engine, session, lowered.symbol, arguments);
        if (answer.failed && answer.failure == "unrepresentable") {
            ++tally.unrepresentable;
            continue;
        }
        ++tally.compared;
        bool agree = false;
        std::string want;
        if (entry.contains("fails")) {
            want = "fails " + entry.at("fails").get<std::string>();
            agree = answer.failed && answer.failure == entry.at("fails").get<std::string>();
        } else {
            want = entry.at("expected").dump();
            agree = !answer.failed && sameValue(answer.value, entry.at("expected"));
        }
        if (agree) {
            ++tally.agreed;
        } else {
            const std::string got = answer.failed ? "fails " + (answer.failure.empty() ? "(unnamed)" : answer.failure) +
                                                        " [" + answer.message + "]"
                                                  : answer.value.dump();
            tally.disagreements.push_back(fixture.name + " case " + std::to_string(index) + " " +
                                          spelled(entry.at("args")) + ": the Interpreter answers " + got +
                                          ", the reference says " + want);
        }
    }
    engine.destroySession(session);
    return tally;
}

}  // namespace

TEST(AnAlgorithmRunsLoweredUnderTheInterpreter, EveryAlgorithmFixtureIsLoweredOrRefusedByName) {
    const nlohmann::json reference = readJson(kReference);
    const std::vector<Fixture> fixtures = algorithmFixtures();
    ASSERT_FALSE(fixtures.empty()) << "the catalog names no algorithm fixture";

    size_t lowered = 0;
    size_t refused = 0;
    size_t casesCompared = 0;
    size_t casesAgreed = 0;
    size_t casesNotAsked = 0;
    size_t casesUnrepresentable = 0;
    std::vector<std::string> disagreements;
    std::vector<std::string> refusals;

    for (const Fixture &fixture : fixtures) {
        const Lowered result = lower(fixture.document);
        if (!result.ok) {
            ++refused;
            // A refusal is the lowering's answer, and it must say what it could
            // not lower: a bare failure is a crash, not a refusal.
            EXPECT_NE(result.refusal.find("generate/unsupported-feature"), std::string::npos)
                << fixture.name << " (" << fixture.document
                << ") was refused without naming a construct: " << result.refusal;
            refusals.push_back(fixture.name + " -> " + result.refusal.substr(0, 160));
            continue;
        }
        ++lowered;
        const auto &section = reference.at(fixture.section);
        ASSERT_TRUE(section.contains(fixture.name)) << fixture.name << " has no cases in " << fixture.section;
        const Tally tally = run(fixture, result, section.at(fixture.name).at("cases"));
        casesCompared += tally.compared;
        casesAgreed += tally.agreed;
        casesNotAsked += tally.notAsked;
        casesUnrepresentable += tally.unrepresentable;
        for (const auto &line : tally.disagreements) {
            disagreements.push_back(line);
        }
        std::cout << "[  LOWERED ] " << fixture.name << ": " << tally.cases << " cases, " << tally.compared
                  << " compared (" << tally.agreed << " agree), " << tally.unrepresentable << " unrepresentable, "
                  << tally.notAsked << " not asked\n";
        // A fixture that lowers and compares nothing is a comparison that was
        // never made, and would read as a pass.
        EXPECT_GT(tally.compared, 0u) << fixture.name << " was lowered and none of its " << tally.cases
                                      << " cases could be compared";
    }

    for (const auto &line : refusals) {
        std::cout << "[  REFUSED ] " << line << "\n";
    }
    std::cout << "[  SUMMARY ] " << lowered << " algorithm fixture(s) lowered, " << refused << " refused; "
              << casesCompared << " case(s) compared, " << casesAgreed << " agree, " << casesUnrepresentable
              << " unrepresentable, " << casesNotAsked << " not asked\n";

    for (const auto &line : disagreements) {
        ADD_FAILURE() << line;
    }
    EXPECT_GT(lowered, 0u) << "no algorithm fixture was lowered, so nothing was compared";
}

}  // namespace Tests
}  // namespace SCE
