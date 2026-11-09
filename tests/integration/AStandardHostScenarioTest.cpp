// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated C++ machine
// sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `SCE::StandardHost::ReferenceHost` registered as the invoker of every standard type.
//
// The scenario is read, and the host set and judged, by the replay the Interpreter's
// shares (integration/StandardHostScenarioReplay.h); this file drives the generated
// machine through it. The machines are generated from the fixtures by
// `tests/CMakeLists.txt`.

#include "common/SceClock.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"
#include "static_host_clock_sm.h"
#include "static_host_http_sm.h"
#include "static_host_kv_sm.h"
#include "static_host_model_sm.h"
#include "static_host_notify_sm.h"
#include "static_model_harness_sm.h"

#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <iterator>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using HostScenario::Rig;
using HostScenario::ScriptTransport;

constexpr int kSettleTicks = 5;

/// Replay the scenario @p stem against a machine of the document @p machine, which the scenario
/// names: a machine whose run ends has one scenario a run (the reference harness's cases).
template <typename Machine>
void replayScenario(const std::string &stem, const std::string &machineName,
                    std::map<std::string, std::function<json(Machine &)>> variables,
                    const std::vector<SCE::AnswerSchema> &schemas) {
    SCOPED_TRACE(stem);
    const json scenario = HostScenario::readScenario(stem);
    ASSERT_EQ(scenario.at("machine").get<std::string>(), machineName)
        << "the scenario " << stem << " names its machine";
    Rig rig;
    rig.host.setAnswerSchemas(schemas.data(), schemas.size());
    Machine machine;
    host::serve(machine, rig.host);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    const auto settle = [&machine]() {
        if constexpr (::SCE::Core::NeedsEventSchedulerTrait<typename Machine::PolicyType>::value) {
            for (int round = 0; round < kSettleTicks; ++round) {
                machine.tick();
            }
        } else {
            machine.step();
        }
    };
    settle();
    HostScenario::HostScenarioDriver driver;
    driver.advance = [&](uint64_t ms) {
        machine.advanceTimeMs(ms);
        settle();
    };
    driver.raise = [&](const std::string &where, const std::string &event, const std::string &data) {
        if (!machine.resolveEventByName(event).has_value()) {
            ADD_FAILURE() << where << ": the machine's events do not match " << event;
            return;
        }
        machine.raiseExternal(event, data);
        settle();
    };
    driver.state = [&]() { return std::string(Machine::PolicyType::getStateName(machine.getCurrentState())); };
    driver.expectVariable = [&](const std::string &where, const std::string &variable, const json &want) {
        const auto read = variables.find(variable);
        if (read == variables.end()) {
            ADD_FAILURE() << where << ": the driver reads no variable " << variable;
            return;
        }
        EXPECT_EQ(read->second(machine), want) << where << ": variable " << variable;
    };
    HostScenario::replay(scenario, rig, driver);
}

/// Replay the scenario of @p name's own name against a machine of that document.
template <typename Machine>
void replay(const std::string &name, std::map<std::string, std::function<json(Machine &)>> variables,
            const std::vector<SCE::AnswerSchema> &schemas = {}) {
    replayScenario<Machine>(name, name, std::move(variables), schemas);
}

}  // namespace

TEST(AStandardHostScenario, AMachineAsksTheKeyValueStoreOfItsHost) {
    using M = SCE::Generated::static_host_kv::static_host_kv;
    replay<M>("static_host_kv", {
                                    {"outcome", [](const M &m) { return json(m.outcome()); }},
                                    {"found", [](const M &m) { return json(m.found()); }},
                                    {"got", [](const M &m) { return json(m.got()); }},
                                    {"replaced", [](const M &m) { return json(m.replaced()); }},
                                    {"existed", [](const M &m) { return json(m.existed()); }},
                                    {"next_found", [](const M &m) { return json(m.next_found()); }},
                                    {"next", [](const M &m) { return json(m.next()); }},
                                });
}

TEST(AStandardHostScenario, AMachineReadsTheClockOfItsHost) {
    using M = SCE::Generated::static_host_clock::static_host_clock;
    replay<M>("static_host_clock", {
                                       {"outcome", [](const M &m) { return json(m.outcome()); }},
                                       {"utc_ms", [](const M &m) { return json(m.utc_ms()); }},
                                       {"monotonic_ms", [](const M &m) { return json(m.monotonic_ms()); }},
                                       {"zone_id", [](const M &m) { return json(m.zone_id()); }},
                                       {"utc_offset_s", [](const M &m) { return json(m.utc_offset_s()); }},
                                   });
}

TEST(AStandardHostScenario, AMachinePostsNotificationsThroughItsHost) {
    using M = SCE::Generated::static_host_notify::static_host_notify;
    replay<M>("static_host_notify", {
                                        {"outcome", [](const M &m) { return json(m.outcome()); }},
                                        {"permission", [](const M &m) { return json(m.permission()); }},
                                        {"status", [](const M &m) { return json(m.status()); }},
                                        {"existed", [](const M &m) { return json(m.existed()); }},
                                    });
}

TEST(AStandardHostScenario, AMachineMakesAnHttpExchangeThroughItsHost) {
    using M = SCE::Generated::static_host_http::static_host_http;
    replay<M>("static_host_http", {
                                      {"outcome", [](const M &m) { return json(m.outcome()); }},
                                      {"status", [](const M &m) { return json(m.status()); }},
                                      {"response_headers", [](const M &m) { return json(m.response_headers()); }},
                                      {"response_body", [](const M &m) { return json(m.response_body()); }},
                                  });
}

namespace {

using ModelMachine = SCE::Generated::static_host_model::static_host_model;

/// The machine's table of answer schemas, as the host is handed it.
std::vector<SCE::AnswerSchema> modelSchemas() {
    using namespace SCE::Generated::static_host_model;
    return {std::begin(ANSWER_SCHEMAS), std::end(ANSWER_SCHEMAS)};
}

/// What static_host_model publishes, read from its saved state, the text every backend saves alike.
json published(ModelMachine &machine, const std::string &variable) {
    return json::parse(machine.saveAt(0).toJson()).at("variables").at(variable);
}

/// Whether a saved value is the scenario's: a 64-bit integer is saved as its digits.
bool holds(const json &saved, const json &expected) {
    if (saved.is_string() && expected.is_number_integer()) {
        return saved.get<std::string>() == expected.dump();
    }
    return saved == expected;
}

/// What the exhaustive provider answers for a class of the table.
host::OutcomeClass outcomeClass(const json &cls) {
    const std::string end = cls.at("end").get<std::string>();
    if (end == "answer") {
        return {host::Generation::Kind::Answer, cls.at("text").get<std::string>(), cls.at("finish").get<std::string>(),
                ""};
    }
    if (end == "failure") {
        return {host::Generation::Kind::Failure, "", "", cls.at("error").get<std::string>()};
    }
    if (end == "refusal") {
        return {host::Generation::Kind::Refusal, "", "", ""};
    }
    return {host::Generation::Kind::Hold, "", "", ""};
}

/// Run static_host_model against the exhaustive provider answering the classes `sequence` names, one call
/// a class, and hold what the machine publishes after each call to what its class says.
void runSequence(const json &table, const std::vector<size_t> &sequence) {
    const json &classes = table.at("classes");
    std::vector<host::OutcomeClass> provided;
    std::string named;
    for (const size_t i : sequence) {
        provided.push_back(outcomeClass(classes.at(i)));
        named += classes.at(i).at("class").get<std::string>() + " ";
    }
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::ExhaustiveProvider provider(provided);
    const auto schemas = modelSchemas();
    reference.setAnswerSchemas(schemas.data(), schemas.size());
    reference.setModelProvider(provider);
    ModelMachine machine;
    host::serve(machine, reference);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    const auto settle = [&machine]() {
        for (int round = 0; round < kSettleTicks; ++round) {
            machine.tick();
        }
    };
    settle();
    const json &ask = table.at("ask");
    json answer = nullptr;
    for (size_t n = 0; n < sequence.size(); ++n) {
        const json &cls = classes.at(sequence[n]);
        machine.raiseExternal(ask.at("event").get<std::string>(), ask.at("data").dump());
        settle();
        const std::string then = cls.value("then", std::string());
        if (then == "advance") {
            machine.advanceTimeMs(table.at("deadline_ms").get<uint64_t>());
            settle();
        } else if (then == "leave") {
            machine.raiseExternal(table.at("leave").get<std::string>(), "");
            settle();
        }
        const json &expect = cls.at("expect");
        if (expect.contains("answer")) {
            answer = expect.at("answer");
        }
        const std::string at = named + "call " + std::to_string(n + 1);
        ASSERT_EQ(ModelMachine::PolicyType::getStateName(machine.getCurrentState()), std::string("idle")) << at;
        for (const char *variable : {"outcome", "why", "finish"}) {
            ASSERT_TRUE(holds(published(machine, variable), expect.at(variable))) << at << ": " << variable;
        }
        ASSERT_TRUE(holds(published(machine, "answer"), answer)) << at << ": answer";
        ASSERT_TRUE(holds(published(machine, "calls"), json(n + 1))) << at << ": calls";
    }
}

}  // namespace

TEST(AStandardHostScenario, AMachineAsksItsHostForATextUnderAnAnswerSchema) {
    std::map<std::string, std::function<json(ModelMachine &)>> variables;
    for (const char *variable : {"calls", "outcome", "finish", "constrained", "answered_by", "prompt_tokens",
                                 "answer_tokens", "answer", "why"}) {
        variables[variable] = [variable](ModelMachine &m) { return published(m, variable); };
    }
    replay<ModelMachine>("static_host_model", variables, modelSchemas());
}

/// The machine around a model is tested exhaustively (docs/adr/0087, section 9.4): every sequence of
/// outcome classes up to the table's bound, each call read as its class says, and every refusal code of
/// the library either reached by a class or named as deferred with its reason.
TEST(AStandardHostScenario, EverySequenceOfOutcomeClassesIsReadAsItsClassesSay) {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" /
                     "model_outcome_classes.json");
    ASSERT_TRUE(in.good()) << "the class table is readable";
    const json table = json::parse(in);
    ASSERT_EQ(table.at("machine").get<std::string>(), "static_host_model");
    ASSERT_EQ(table.at("answer_schema").get<std::string>(), std::string(modelSchemas().at(0).id));
    const size_t count = table.at("classes").size();
    const size_t bound = table.at("bound").get<size_t>();
    ASSERT_TRUE(count > 0 && bound > 0) << "a table that names nothing judges nothing";
    size_t runs = 0;
    size_t expected = 0;
    size_t power = 1;
    for (size_t length = 1; length <= bound; ++length) {
        power *= count;
        expected += power;
        std::vector<size_t> sequence(length, 0);
        while (true) {
            runSequence(table, sequence);
            if (::testing::Test::HasFatalFailure()) {
                return;
            }
            ++runs;
            size_t position = length;
            while (position > 0 && sequence[position - 1] == count - 1) {
                sequence[--position] = 0;
            }
            if (position == 0) {
                break;
            }
            ++sequence[position - 1];
        }
    }
    EXPECT_EQ(runs, expected) << "every sequence up to the bound was run";
    std::set<std::string> codes;
    for (const auto &cls : table.at("classes")) {
        if (cls.at("expect").at("why").is_object()) {
            codes.insert(cls.at("expect").at("why").at("code").get<std::string>());
        }
    }
    for (const auto &deferred : table.at("deferred")) {
        codes.insert(deferred.at("code").get<std::string>());
    }
    // The fourteen codes, tooFew the last (docs/adr/0086, section 15.11).
    EXPECT_EQ(codes.size(), 14u) << "every refusal code is reached or deferred";
}

namespace {

using HarnessMachine = SCE::Generated::static_model_harness::static_model_harness;

/// The cases of the reference harness's scenario (docs/adr/0087, section 9.1 item 8), one scenario each:
/// a run of the harness ends at one of its finals, so each case is a run of its own.
constexpr const char *kHarnessScenarios[] = {"static_model_harness_accepted", "static_model_harness_exhausted",
                                             "static_model_harness_not_offered", "static_model_harness_repaired",
                                             "static_model_harness_timeout"};

/// The codes of the application's own checks, after the fourteen of the lift its enum repeats
/// (enum_harness_code.scxml), each named for the class that reaches it.
constexpr const char *kApplicationCodes[] = {"notOffered"};

/// The ticks a harness run is given after each event: a run may make its calls and take each completion
/// within one event, a call a round.
constexpr int kHarnessSettleTicks = 8;

/// The harness's table of answer schemas, as the host is handed it.
std::vector<SCE::AnswerSchema> harnessSchemas() {
    using namespace SCE::Generated::static_model_harness;
    return {std::begin(ANSWER_SCHEMAS), std::end(ANSWER_SCHEMAS)};
}

/// What the harness holds, read from its saved state, the text every backend saves alike.
json harnessVariable(HarnessMachine &machine, const std::string &variable) {
    return json::parse(machine.saveAt(0).toJson()).at("variables").at(variable);
}

/// A row of the harness's transition table as a failure names it: its state's id and its index.
std::string harnessRowName(HarnessMachine::State source, std::uint32_t index) {
    return std::string(HarnessMachine::PolicyType::getStateName(source)) + "[" + std::to_string(index) + "]";
}

/// The end of the host operation a class gives a call (docs/adr/0087, section 9.4): an answer by its
/// `finish`, a failure by its code, the refusal, and a held call, whichever way it is left.
std::string hostOutcome(const json &cls) {
    const std::string end = cls.at("end").get<std::string>();
    if (end == "answer") {
        return "answer:" + cls.at("finish").get<std::string>();
    }
    if (end == "failure") {
        return cls.at("error").get<std::string>();
    }
    if (end == "refusal") {
        return "invalid_argument";
    }
    return "hold";
}

/// What one run of the harness against the exhaustive provider showed.
struct HarnessRun {
    /// Every row the transition observer was told, in order.
    std::vector<HarnessMachine::TransitionTaken> rows;
    /// How many calls the host was asked to start.
    size_t calls = 0;
    /// The state the run ended in.
    std::string ended;
    /// The code of each refusal the harness kept, in order.
    std::vector<std::string> codes;
};

/// Run the reference harness against the exhaustive provider answering the classes `sequence` names, one
/// call a class: offer it the table's ids, start it, and leave each call it holds as the table's harness
/// section says, until it ends.
HarnessRun runHarness(const json &table, const std::vector<size_t> &sequence) {
    const json &classes = table.at("classes");
    const json &harness = table.at("harness");
    EXPECT_EQ(harness.at("then").get<std::string>(), "advance") << "a held call is left by its own deadline";
    std::vector<host::OutcomeClass> provided;
    for (const size_t i : sequence) {
        provided.push_back(outcomeClass(classes.at(i)));
    }
    HarnessRun run;
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::ExhaustiveProvider provider(provided);
    const auto schemas = harnessSchemas();
    reference.setAnswerSchemas(schemas.data(), schemas.size());
    reference.setModelProvider(provider);
    HarnessMachine machine;
    machine.setTransitionObserver([&run](const HarnessMachine::TransitionTaken &taken) { run.rows.push_back(taken); });
    host::serve(machine, reference);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    const auto settle = [&machine]() {
        for (int round = 0; round < kHarnessSettleTicks; ++round) {
            machine.tick();
        }
    };
    settle();
    for (const auto &id : harness.at("offered")) {
        machine.raiseExternal(harness.at("offer").get<std::string>(), json{{"id", id}}.dump());
        settle();
    }
    machine.raiseExternal(harness.at("start").get<std::string>(), harness.at("data").dump());
    settle();
    const auto current = [&machine]() {
        return std::string(HarnessMachine::PolicyType::getStateName(machine.getCurrentState()));
    };
    const size_t bound = harness.at("bound").get<size_t>();
    for (size_t waits = 0; current() != "accepted" && current() != "gave_up"; ++waits) {
        if (waits > bound) {
            ADD_FAILURE() << "the run neither ended nor held a call it could be moved past";
            break;
        }
        machine.advanceTimeMs(harness.at("deadline_ms").get<uint64_t>());
        settle();
    }
    run.calls = reference.takeStarts().size();
    run.ended = current();
    for (const auto &refusal : harnessVariable(machine, "refusals")) {
        run.codes.push_back(refusal.at("code").get<std::string>());
    }
    return run;
}

}  // namespace

TEST(AStandardHostScenario, TheReferenceHarnessRunsEachCaseOfItsScenario) {
    std::map<std::string, std::function<json(HarnessMachine &)>> variables;
    for (const char *variable : {"outcome", "attempts", "offered", "refusals", "answer", "why"}) {
        variables[variable] = [variable](HarnessMachine &m) { return harnessVariable(m, variable); };
    }
    for (const char *stem : kHarnessScenarios) {
        replayScenario<HarnessMachine>(stem, "static_model_harness", variables, harnessSchemas());
    }
}

/// The reference harness is tested exhaustively, and its coverage is measured (docs/adr/0087, section 9.4
/// item 1, Landing order item 7): every sequence of `bound` outcome classes is a run, the transition
/// observer of docs/adr/0089 tells each row of the emitted transition table the run takes, and the runs
/// together take every row of the table, reach every refusal code, and reach every host outcome at every
/// call site (a row that enters the state whose invoke is the call).
///
/// The harness makes at most `bound` calls and the provider answers a call by its position alone, so every
/// sequence of fewer classes the harness can consume is a prefix of a sequence run here, and none is asked
/// past.
TEST(AStandardHostScenario, EverySequenceOfOutcomeClassesTakesEveryRowOfTheReferenceHarness) {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" /
                     "model_outcome_classes.json");
    ASSERT_TRUE(in.good()) << "the class table is readable";
    const json table = json::parse(in);
    const json &harness = table.at("harness");
    ASSERT_EQ(harness.at("machine").get<std::string>(), "static_model_harness");
    ASSERT_EQ(table.at("answer_schema").get<std::string>(), std::string(harnessSchemas().at(0).id));
    const json &classes = table.at("classes");
    const size_t count = classes.size();
    const size_t bound = harness.at("bound").get<size_t>();
    const std::string call = harness.at("call").get<std::string>();
    ASSERT_TRUE(count > 0 && bound > 0) << "a table that names nothing judges nothing";
    std::set<std::string> rows;
    std::set<std::string> sites;
    for (const auto &row : HarnessMachine::PolicyType::transitions) {
        rows.insert(harnessRowName(row.from, row.index));
        if (call == row.targets) {
            sites.insert(harnessRowName(row.from, row.index));
        }
    }
    ASSERT_FALSE(sites.empty()) << "no row of the table enters " << call;
    std::set<std::string> outcomes;
    for (const auto &cls : classes) {
        outcomes.insert(hostOutcome(cls));
    }

    std::set<std::string> taken;
    std::set<std::string> codes;
    std::set<std::string> reached;
    size_t runs = 0;
    size_t expected = 1;
    for (size_t i = 0; i < bound; ++i) {
        expected *= count;
    }
    std::vector<size_t> sequence(bound, 0);
    while (true) {
        std::string named;
        for (const size_t i : sequence) {
            named += classes.at(i).at("class").get<std::string>() + " ";
        }
        const HarnessRun run = runHarness(table, sequence);
        if (::testing::Test::HasFailure()) {
            FAIL() << named << ": the run could not be made";
        }
        ASSERT_TRUE(run.ended == "accepted" || run.ended == "gave_up")
            << named << ": the run ended in " << run.ended << ", not a final";
        std::vector<std::string> called;
        for (const auto &row : run.rows) {
            const std::string name = harnessRowName(row.source, row.index);
            if (sites.count(name) != 0) {
                called.push_back(name);
            }
            taken.insert(name);
        }
        ASSERT_TRUE(called.size() == run.calls && run.calls <= bound)
            << named << ": " << run.calls << " calls, " << called.size() << " of them by a row that enters " << call;
        for (size_t n = 0; n < called.size(); ++n) {
            reached.insert(called[n] + " " + hostOutcome(classes.at(sequence[n])));
        }
        codes.insert(run.codes.begin(), run.codes.end());
        ++runs;
        size_t position = bound;
        while (position > 0 && sequence[position - 1] == count - 1) {
            sequence[--position] = 0;
        }
        if (position == 0) {
            break;
        }
        ++sequence[position - 1];
    }
    EXPECT_EQ(runs, expected) << "every sequence was run";

    std::string toldNoRow;
    for (const auto &row : taken) {
        if (rows.count(row) == 0) {
            toldNoRow += row + " ";
        }
    }
    EXPECT_TRUE(toldNoRow.empty()) << "the observer was told pairs that are no row of the table: " << toldNoRow;
    std::string neverTaken;
    for (const auto &row : rows) {
        if (taken.count(row) == 0) {
            neverTaken += row + " ";
        }
    }
    EXPECT_TRUE(neverTaken.empty()) << "every row of the harness's transition table is taken; never taken: "
                                    << neverTaken;

    std::set<std::string> deferred;
    for (const auto &entry : table.at("deferred")) {
        deferred.insert(entry.at("code").get<std::string>());
    }
    std::set<std::string> expectedCodes(std::begin(kApplicationCodes), std::end(kApplicationCodes));
    expectedCodes.insert(deferred.begin(), deferred.end());
    for (const auto &cls : classes) {
        if (cls.at("expect").at("why").is_object()) {
            expectedCodes.insert(cls.at("expect").at("why").at("code").get<std::string>());
        }
    }
    EXPECT_EQ(expectedCodes.size(), 15u) << "the fourteen lift codes and the application's own";
    std::string neverReached;
    for (const auto &code : expectedCodes) {
        if (codes.count(code) == 0 && deferred.count(code) == 0) {
            neverReached += code + " ";
        }
    }
    EXPECT_TRUE(neverReached.empty()) << "every refusal code is reached; never reached: " << neverReached;
    std::string unknown;
    for (const auto &code : codes) {
        if (expectedCodes.count(code) == 0) {
            unknown += code + " ";
        }
    }
    EXPECT_TRUE(unknown.empty()) << "the harness kept codes no class reaches: " << unknown;

    std::string notAtEverySite;
    for (const auto &site : sites) {
        for (const auto &outcome : outcomes) {
            if (reached.count(site + " " + outcome) == 0) {
                notAtEverySite += site + " " + outcome + "; ";
            }
        }
    }
    EXPECT_TRUE(notAtEverySite.empty()) << "every host outcome is reached at every call site; never reached: "
                                        << notAtEverySite;
}

TEST(AStandardHostScenario, TheReferenceHostJudgesAGenerationBeforeItsProvider) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::ScriptedProvider provider;
    provider.constrained = true;
    provider.context = 10;
    provider.answers.push_back({std::string(host::ANSWER_MAX_BYTES + 1, 'a'), "stop"});
    provider.answers.push_back({"kept", "stop"});
    reference.setModelProvider(provider);
    const SCE::AnswerSchema schemas[] = {{"urn:sce:schema:answer:1", "{}", "root ::= \"{}\""}};
    reference.setAnswerSchemas(schemas);
    const auto ask = [&reference](const std::string &prompt, const std::string &schema, const std::string &maxTokens,
                                  const std::string &temperature) {
        return reference.start(host::MODEL_GENERATE, "gen",
                               {{"prompt", prompt},
                                {"answer_schema", schema},
                                {"max_tokens", maxTokens},
                                {"temperature_milli", temperature}});
    };
    EXPECT_EQ(ask("", "", "8", "0")->kind, host::Outcome::Kind::Refuse);
    EXPECT_EQ(ask("p", "", "0", "0")->kind, host::Outcome::Kind::Refuse);
    EXPECT_EQ(ask("p", "", "8", "2001")->kind, host::Outcome::Kind::Refuse);
    EXPECT_EQ(ask("p", "urn:sce:schema:other:1", "8", "0")->kind, host::Outcome::Kind::Refuse);
    EXPECT_EQ(json::parse(ask("p", "", "8", "0")->data).at("error"), "answer_too_large");
    EXPECT_EQ(json::parse(ask("abc", "", "8", "0")->data).at("error"), "context_exceeded");
    const json kept = json::parse(ask("abc", "urn:sce:schema:answer:1", "7", "2000")->data);
    EXPECT_EQ(kept.at("text"), "kept");
    EXPECT_EQ(kept.at("constrained"), true);
    EXPECT_EQ(kept.at("prompt_tokens"), 3);
    EXPECT_EQ(json::parse(ask("p", "", "8", "0")->data).at("error"), "unavailable");
}

TEST(AStandardHostScenario, EveryScenarioOfTheDirectoryIsReplayedHere) {
    std::vector<std::string> replayed{"static_host_clock", "static_host_http", "static_host_kv", "static_host_model",
                                      "static_host_notify"};
    replayed.insert(replayed.end(), std::begin(kHarnessScenarios), std::end(kHarnessScenarios));
    std::sort(replayed.begin(), replayed.end());
    EXPECT_EQ(HostScenario::scenarioNames(), replayed);
}

TEST(AStandardHostScenario, TheSystemClockAnswersAMonotonicReadingThatDoesNotDecrease) {
    host::SystemClock clock;
    host::ScriptedClock unused;

    struct Unreachable : host::Transport {
        host::Exchange exchange(const host::HttpRequest &) override {
            return {host::Exchange::Kind::Failure, {}};
        }
    } transport;

    host::ReferenceHost system(clock, transport);
    int64_t last = -1;
    for (int i = 0; i < 50; ++i) {
        const auto outcome = system.start("sce-host-clock-now", "now", {});
        ASSERT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done);
        const json fields = json::parse(outcome->data);
        EXPECT_EQ(fields.at("error"), "");
        const int64_t mono = std::stoll(fields.at("monotonic_ms").get<std::string>());
        EXPECT_GE(mono, last);
        last = mono;
    }
    const auto zone = system.start("sce-host-clock-zone", "zone", {});
    ASSERT_TRUE(zone && zone->kind == host::Outcome::Kind::Done);
    EXPECT_EQ(json::parse(zone->data).at("error"), "");
}

TEST(AStandardHostScenario, AnOperationOutsideTheStandardSetIsNotServedAndANulKeyIsRefused) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference(clock, transport);
    EXPECT_FALSE(reference.start("x-app-host", "id", {}).has_value());
    const auto nul = reference.start("sce-host-kv-get", "id", {{"key", std::string("a\0b", 3)}});
    ASSERT_TRUE(nul.has_value());
    EXPECT_EQ(nul->kind, host::Outcome::Kind::Refuse);
}

/// Keys are ordered by their UTF-8 bytes.
TEST(AStandardHostScenario, KeysAreOrderedByTheirUtf8Bytes) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference(clock, transport);
    for (const char *key : {"\xF0\x9F\x98\x80", "\xEF\xBD\x9E"}) {
        reference.start("sce-host-kv-put", "put", {{"key", key}, {"value", "v"}});
    }
    const auto walk = reference.start("sce-host-kv-next-key", "walk", {{"after", ""}, {"prefix", ""}});
    ASSERT_TRUE(walk.has_value());
    EXPECT_EQ(json::parse(walk->data).at("key"), "\xEF\xBD\x9E");
}

}  // namespace SCE::Tests
