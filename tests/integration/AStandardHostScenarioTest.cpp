// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

// The machine of the COSE operations is generated, and its scenarios replayed, in a build whose
// reference host has them (cmake/SCEHostCose.cmake defines SCE_HOST_COSE where it found libsodium).
#if defined(SCE_HOST_COSE)
#include "static_host_cose_sm.h"
#endif

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <iterator>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
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

/// Run @p machine until what it was handed is taken: a round of its scheduler's ticks where it keeps one, a
/// step otherwise.
template <typename Machine> void settleMachine(Machine &machine) {
    if constexpr (::SCE::Core::NeedsEventSchedulerTrait<typename Machine::PolicyType>::value) {
        for (int round = 0; round < kSettleTicks; ++round) {
            machine.tick();
        }
    } else {
        machine.step();
    }
}

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
    const auto settle = [&machine]() { settleMachine(machine); };
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
                                    {"got_version", [](const M &m) { return json(m.got_version()); }},
                                    {"replaced", [](const M &m) { return json(m.replaced()); }},
                                    {"written", [](const M &m) { return json(m.written()); }},
                                    {"put_version", [](const M &m) { return json(m.put_version()); }},
                                    {"existed", [](const M &m) { return json(m.existed()); }},
                                    {"next_found", [](const M &m) { return json(m.next_found()); }},
                                    {"next", [](const M &m) { return json(m.next()); }},
                                });
}

// docs/adr/0107, D4: a put-if the host carried out, saved before its answer was taken, is
// started again by the restore under the identity it had, and the host answers it with the first
// result rather than judging the store a second time; a start the document makes afterwards takes
// the next number and is an operation of its own.
TEST(AStandardHostScenario, ARestoredPutIfIsAnsweredWithTheFirstResultOfItsInvocation) {
    using M = SCE::Generated::static_host_kv::static_host_kv;
    ScriptTransport transport;
    host::ScriptedClock clock;
    host::ReferenceHost reference(clock, transport);
    const auto settle = [](M &machine) {
        if constexpr (::SCE::Core::NeedsEventSchedulerTrait<typename M::PolicyType>::value) {
            for (int round = 0; round < kSettleTicks; ++round) {
                machine.tick();
            }
        } else {
            machine.step();
        }
    };

    // The first process: the host writes, and its answer is lost with the process before the
    // machine takes it.
    M first;
    host::serve(first, reference);
    first.registerInvoker(
        "sce-host-kv-put-if",
        [&reference](const SCE::HostInvokeEvent &event) -> std::optional<SCE::HostInvokeResponse> {
            if (event.start.has_value()) {
                host::Params params;
                for (const auto &[name, values] : event.start->params) {
                    if (!values.empty()) {
                        params[name] = values.front();
                    }
                }
                static_cast<void>(reference.start(
                    "sce-host-kv-put-if",
                    host::Invocation{event.start->sessionId, event.start->invokeId, event.start->start}, params));
            }
            return std::nullopt;
        });
    first.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(first.initialize(), std::nullopt);
    settle(first);
    first.raiseExternal("host.kv.put_if.request", R"({"key":"k","value":"a","expected_version":"0"})");
    settle(first);
    const SCE::Saved::SavedState saved = first.saveAt(0);
    ASSERT_EQ(saved.hostInvokes.size(), 1u) << "the put-if is running";
    EXPECT_EQ(saved.hostInvokes[0].invokeId, "putif");
    EXPECT_EQ(saved.hostInvokes[0].start, 1);
    const std::map<std::string, std::string> once = {{"k", "a"}};
    EXPECT_EQ(reference.storeEntries(), once) << "the host wrote once";

    // The second process: the restore starts it again, and the host answers it as it answered
    // the first time, where the store alone would now answer `written` false at version 1.
    M second;
    second.setClock(std::make_shared<SCE::ManualClock>(0));
    second.restoreAt(SCE::Saved::SavedState::fromJson(saved.toJson()), 0);
    host::serve(second, reference);
    settle(second);
    EXPECT_EQ(json(second.outcome()), json("done"));
    EXPECT_EQ(json(second.written()), json(true)) << "the first result";
    EXPECT_EQ(json(second.put_version()), json("1"));
    EXPECT_EQ(reference.storeEntries(), once) << "nothing written twice";

    // A start the document makes now is the second start of `putif`.
    second.raiseExternal("host.kv.put_if.request", R"({"key":"k","value":"b","expected_version":"0"})");
    settle(second);
    EXPECT_EQ(json(second.written()), json(false)) << "judged against the store as it is";
    EXPECT_EQ(json(second.put_version()), json("1"));
    const std::vector<host::Start> starts = reference.takeStarts();
    ASSERT_EQ(starts.size(), 3u);
    for (std::size_t at = 0; at < starts.size(); ++at) {
        EXPECT_EQ(starts[at].id, "putif");
        EXPECT_EQ(starts[at].sessionId, saved.sessionId);
        EXPECT_EQ(starts[at].number, at < 2 ? 1u : 2u) << "start " << at;
    }
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
        return reference.start(host::MODEL_GENERATE, host::Invocation{"session", "gen", 1},
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

#if defined(SCE_HOST_COSE)

namespace {

using CoseMachine = SCE::Generated::static_host_cose::static_host_cose;

/// A reader of each variable a step of @p scenario expects, from the machine's saved state, the text
/// every backend saves alike.
std::map<std::string, std::function<json(CoseMachine &)>> coseVariables(const json &scenario) {
    std::map<std::string, std::function<json(CoseMachine &)>> variables;
    for (const auto &step : scenario.at("steps")) {
        const json &expect = step.at("expect");
        if (!expect.contains("variables")) {
            continue;
        }
        for (const auto &entry : expect.at("variables").items()) {
            const std::string variable = entry.key();
            variables[variable] = [variable](CoseMachine &m) {
                return json::parse(m.saveAt(0).toJson()).at("variables").at(variable);
            };
        }
    }
    return variables;
}

}  // namespace

/// The COSE operations (docs/adr/0107, D7): every scenario that names static_host_cose, replayed against
/// it with the reference host's COSE feature on libsodium, its keys set by each scenario's `cose_keys`.
TEST(AStandardHostScenario, AMachineSignsSealsAndDerivesKeysThroughItsHost) {
    const std::vector<std::string> stems = HostScenario::scenariosOf(HostScenario::COSE_MACHINE);
    ASSERT_FALSE(stems.empty()) << "no scenario names " << HostScenario::COSE_MACHINE;
    for (const auto &stem : stems) {
        replayScenario<CoseMachine>(stem, HostScenario::COSE_MACHINE, coseVariables(HostScenario::readScenario(stem)),
                                    {});
    }
}

namespace {

namespace cose = SCE::Generated::static_host_cose;

/// A host of key generation through the generated typed interface: records each request it is handed and
/// answers it with a key id at once.
class RecordingKeyGeneration : public cose::SceHostKeyGenerateInvoker {
public:
    std::vector<cose::KeyGenerateRequest> requests;

    std::optional<cose::KeyGenerateResult> startKeyGenerate(const cose::KeyGenerateRequest &request,
                                                            uint64_t) override {
        requests.push_back(request);
        cose::KeyGenerateResult result;
        result.key_id = "generated:k";
        return result;
    }

    void cancelKeyGenerate(uint64_t) override {}
};

}  // namespace

/// docs/adr/0109, step 4 of its order of work: a key's use is a value of the library's enum `key_use`, which
/// crosses to the host as the name the enum declares for it, and the generated adapter reads it back into the
/// machine's own enum by that name. The Rust twin is a_typed_request_reads_its_enum_field_by_its_name.
TEST(AStandardHostScenario, ATypedKeyGenerationReadsItsUseByItsName) {
    CoseMachine machine;
    const auto host = std::make_shared<RecordingKeyGeneration>();
    machine.registerSceHostKeyGenerateInvoker(host);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    EXPECT_EQ(machine.initialize(), std::nullopt);
    settleMachine(machine);
    const std::vector<std::pair<std::string, cose::StaticHostCoseKeyUseEnum>> uses{
        {"sign", cose::StaticHostCoseKeyUseEnum::Sign},
        {"seal", cose::StaticHostCoseKeyUseEnum::Seal},
        {"mac", cose::StaticHostCoseKeyUseEnum::Mac},
        {"derive", cose::StaticHostCoseKeyUseEnum::Derive},
    };
    for (const auto &use : uses) {
        machine.raiseExternal("host.key.generate.request", json{{"key_use", use.first}, {"label", "device"}}.dump());
        settleMachine(machine);
        settleMachine(machine);
    }
    ASSERT_EQ(host->requests.size(), uses.size()) << "each use reached the host once";
    for (size_t i = 0; i < uses.size(); ++i) {
        EXPECT_TRUE(host->requests[i].key_use == uses[i].second) << "the use the host was handed for " << uses[i].first;
        EXPECT_EQ(host->requests[i].label, "device");
    }
}

#endif

/// Every scenario of the directory is replayed by a test of this file. Those of the COSE operations are
/// replayed by AMachineSignsSealsAndDerivesKeysThroughItsHost in a build that found libsodium; a build that
/// did not serves none of them (docs/adr/0107, D7). Either way they are told apart by the machine they name.
TEST(AStandardHostScenario, EveryScenarioOfTheDirectoryIsReplayedHere) {
    std::vector<std::string> replayed{"static_host_clock", "static_host_http", "static_host_kv", "static_host_model",
                                      "static_host_notify"};
    replayed.insert(replayed.end(), std::begin(kHarnessScenarios), std::end(kHarnessScenarios));
    const std::vector<std::string> cose = HostScenario::scenariosOf(HostScenario::COSE_MACHINE);
    replayed.insert(replayed.end(), cose.begin(), cose.end());
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
        const auto outcome = system.start("sce-host-clock-now", host::Invocation{"session", "now", 1}, {});
        ASSERT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done);
        const json fields = json::parse(outcome->data);
        EXPECT_EQ(fields.at("error"), "");
        const int64_t mono = std::stoll(fields.at("monotonic_ms").get<std::string>());
        EXPECT_GE(mono, last);
        last = mono;
    }
    const auto zone = system.start("sce-host-clock-zone", host::Invocation{"session", "zone", 1}, {});
    ASSERT_TRUE(zone && zone->kind == host::Outcome::Kind::Done);
    EXPECT_EQ(json::parse(zone->data).at("error"), "");
}

/// The system clock's zone (docs/SCE_HOST_FEATURES.md, section 2) on every platform: an offset some zone
/// is at today, whole minutes between UTC-12 and UTC+14, and an identifier that is empty or names a zone
/// within the 64 characters the result carries.
TEST(AStandardHostScenario, TheSystemClockAnswersAnOffsetAZoneHasAndAnIdentifierOrNone) {
    host::SystemClock clock;
    const auto zone = clock.zone();
    ASSERT_TRUE(zone.has_value());
    EXPECT_GE(zone->utcOffsetS, -12 * 3600);
    EXPECT_LE(zone->utcOffsetS, 14 * 3600);
    EXPECT_EQ(zone->utcOffsetS % 60, 0);
    EXPECT_LE(zone->id.size(), 64u);
    if (!zone->id.empty()) {
        EXPECT_TRUE(zone->id == "UTC" || zone->id.find('/') != std::string::npos) << zone->id;
    }
}

#if defined(_WIN32) && defined(__cpp_lib_chrono) && __cpp_lib_chrono >= 201907L
/// On Windows the standard library's own time zone, which reads the zone Windows is set to through the
/// same system ICU, is the oracle: the clock answers the offset it says is in force now and, where the
/// clock reaches ICU, names the zone it names.
TEST(AStandardHostScenario, TheSystemClockOnWindowsAgreesWithTheStandardLibrarysCurrentZone) {
    host::SystemClock clock;
    const auto zone = clock.zone();
    ASSERT_TRUE(zone.has_value());
    const std::chrono::time_zone *current = std::chrono::current_zone();
    const std::chrono::sys_info now = current->get_info(std::chrono::system_clock::now());
    EXPECT_EQ(zone->utcOffsetS, static_cast<int32_t>(now.offset.count()));
#if defined(SCE_STANDARD_HOST_SYSTEM_ICU)
    ASSERT_FALSE(zone->id.empty());
    EXPECT_EQ(std::chrono::locate_zone(zone->id), current) << zone->id;
#endif
}
#endif

TEST(AStandardHostScenario, AnOperationOutsideTheStandardSetIsNotServedAndANulKeyIsRefused) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference(clock, transport);
    EXPECT_FALSE(reference.start("x-app-host", host::Invocation{"session", "id", 1}, {}).has_value());
    const auto nul =
        reference.start("sce-host-kv-get", host::Invocation{"session", "id", 2}, {{"key", std::string("a\0b", 3)}});
    ASSERT_TRUE(nul.has_value());
    EXPECT_EQ(nul->kind, host::Outcome::Kind::Refuse);
}

/// A put-if is refused for a key with U+0000 as a get is, and writes nothing.
TEST(AStandardHostScenario, APutIfWithANulInItsKeyIsRefused) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference(clock, transport);
    const auto nul = reference.start("sce-host-kv-put-if", host::Invocation{"session", "putif", 1},
                                     {{"key", std::string("a\0b", 3)}, {"value", "v"}, {"expected_version", "0"}});
    ASSERT_TRUE(nul.has_value());
    EXPECT_EQ(nul->kind, host::Outcome::Kind::Refuse);
    const auto entries = reference.storeEntries();
    ASSERT_TRUE(entries.has_value());
    EXPECT_TRUE(entries->empty());
}

/// docs/adr/0109, D13: a host built on a store the embedding gives keeps its values and the first result
/// of each put-if there, so a host of the next process on the same store answers a put-if the restore
/// starts again with its first result, and writes nothing.
TEST(AStandardHostScenario, AHostOnTheStoreItWasGivenAnswersARepeatedPutIfWithItsFirstResult) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::MemoryKvStore store;
    const host::Invocation putIf{"session-1", "putif", 1};
    const host::Params params = {{"key", "k"}, {"value", "a"}, {"expected_version", "0"}};
    std::string first;
    {
        host::ReferenceHost before(clock, transport, store);
        const auto wrote = before.start("sce-host-kv-put-if", putIf, params);
        ASSERT_TRUE(wrote.has_value());
        first = wrote->data;
        EXPECT_EQ(json::parse(first).at("written"), true);
    }
    host::ReferenceHost after(clock, transport, store);
    const std::map<std::string, std::string> once = {{"k", "a"}};
    EXPECT_EQ(after.storeEntries(), once) << "the store outlived the host that wrote";
    const auto again = after.start("sce-host-kv-put-if", putIf, params);
    ASSERT_TRUE(again.has_value());
    EXPECT_EQ(again->data, first) << "the first result, where the store alone answers a lost race";
    EXPECT_EQ(after.storeEntries(), once) << "nothing written twice";
    const auto next = after.start("sce-host-kv-put-if", host::Invocation{"session-1", "putif", 2}, params);
    ASSERT_TRUE(next.has_value());
    EXPECT_EQ(json::parse(next->data).at("written"), false) << "the next start is judged against the store";

    // A failure the embedding injected is a first result too.
    after.setFaults({{"sce-host-kv-put-if", "unavailable"}});
    const host::Invocation failed{"session-1", "putif", 3};
    const auto injected = after.start("sce-host-kv-put-if", failed, params);
    ASSERT_TRUE(injected.has_value());
    EXPECT_EQ(json::parse(injected->data).at("error"), "unavailable");
    after.setFaults({});
    const auto repeated = after.start("sce-host-kv-put-if", failed, params);
    ASSERT_TRUE(repeated.has_value());
    EXPECT_EQ(repeated->data, injected->data) << "the repeat is answered with the injected failure";
}

/// Keys are ordered by their UTF-8 bytes.
TEST(AStandardHostScenario, KeysAreOrderedByTheirUtf8Bytes) {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost reference(clock, transport);
    for (const char *key : {"\xF0\x9F\x98\x80", "\xEF\xBD\x9E"}) {
        reference.start("sce-host-kv-put", host::Invocation{"session", "put", 1}, {{"key", key}, {"value", "v"}});
    }
    const auto walk = reference.start("sce-host-kv-next-key", host::Invocation{"session", "walk", 1},
                                      {{"after", ""}, {"prefix", ""}});
    ASSERT_TRUE(walk.has_value());
    EXPECT_EQ(json::parse(walk->data).at("key"), "\xEF\xBD\x9E");
}

}  // namespace SCE::Tests
