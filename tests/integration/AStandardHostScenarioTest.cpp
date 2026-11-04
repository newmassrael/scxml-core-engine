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
#include "host/HttplibTransport.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"
#include "static_host_clock_sm.h"
#include "static_host_http_sm.h"
#include "static_host_kv_sm.h"
#include "static_host_model_sm.h"
#include "static_host_notify_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <httplib.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <set>
#include <string>
#include <thread>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using HostScenario::Rig;
using HostScenario::ScriptTransport;

constexpr int kSettleTicks = 5;

template <typename Machine>
void replay(const std::string &name, std::map<std::string, std::function<json(Machine &)>> variables,
            const std::vector<SCE::AnswerSchema> &schemas = {}) {
    const json scenario = HostScenario::readScenario(name);
    ASSERT_EQ(scenario.at("machine").get<std::string>(), name);
    Rig rig;
    rig.host.setAnswerSchemas(schemas.data(), schemas.size());
    Machine machine;
    host::serve(machine, rig.host);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.initialize();
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
    machine.initialize();
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
    EXPECT_EQ(codes.size(), 13u) << "every refusal code is reached or deferred";
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
    EXPECT_EQ(HostScenario::scenarioNames(),
              (std::vector<std::string>{"static_host_clock", "static_host_http", "static_host_kv", "static_host_model",
                                        "static_host_notify"}));
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

/// The transport over cpp-httplib: a local server answers, the host carries the request's headers and
/// body to it, follows no redirect, and gives the response's fields in a fixed order.
TEST(AStandardHostScenario, TheHttplibTransportCarriesAnExchangeToALocalServer) {
    httplib::Server server;
    std::string seenBody;
    std::string seenTrace;
    server.Post("/items", [&](const httplib::Request &request, httplib::Response &response) {
        seenBody = request.body;
        seenTrace = request.get_header_value("X-Trace");
        response.status = 201;
        response.set_header("X-Reply", "yes");
        response.set_content("made", "text/plain");
    });
    server.Get("/moved", [](const httplib::Request &, httplib::Response &response) {
        response.status = 301;
        response.set_header("Location", "/elsewhere");
    });
    server.Get("/missing", [](const httplib::Request &, httplib::Response &response) { response.status = 404; });
    const int port = server.bind_to_any_port("127.0.0.1");
    ASSERT_GT(port, 0);
    std::thread serving([&server] { server.listen_after_bind(); });
    server.wait_until_ready();
    const std::string base = "http://127.0.0.1:" + std::to_string(port);

    host::ScriptedClock clock;
    host::HttplibTransport transport;
    host::ReferenceHost reference(clock, transport);
    const auto fetch = [&reference](const std::string &method, const std::string &url, const std::string &headers = "",
                                    const std::string &body = "") {
        const auto outcome = reference.start("sce-host-http-fetch", "fetch",
                                             {{"method", method}, {"url", url}, {"headers", headers}, {"body", body}});
        EXPECT_TRUE(outcome && outcome->kind == host::Outcome::Kind::Done);
        return json::parse(outcome->data);
    };

    const json made = fetch("POST", base + "/items", "X-Trace: 7\nContent-Type: text/plain", "payload");
    EXPECT_EQ(made.at("error"), "");
    EXPECT_EQ(made.at("status"), 201);
    EXPECT_EQ(made.at("body"), "made");
    EXPECT_NE(made.at("headers").get<std::string>().find("x-reply: yes"), std::string::npos);
    EXPECT_EQ(seenBody, "payload");
    EXPECT_EQ(seenTrace, "7");
    const json moved = fetch("GET", base + "/moved");
    EXPECT_EQ(moved.at("status"), 301);
    EXPECT_NE(moved.at("headers").get<std::string>().find("location: /elsewhere"), std::string::npos);
    EXPECT_EQ(fetch("GET", base + "/missing").at("status"), 404);

    server.stop();
    serving.join();
    EXPECT_EQ(fetch("GET", base + "/items").at("error"), "network");
}

}  // namespace SCE::Tests
