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
#include <string>
#include <thread>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using HostScenario::Rig;
using HostScenario::ScriptTransport;

constexpr int kSettleTicks = 5;

template <typename Machine>
void replay(const std::string &name, std::map<std::string, std::function<json(const Machine &)>> variables) {
    const json scenario = HostScenario::readScenario(name);
    ASSERT_EQ(scenario.at("machine").get<std::string>(), name);
    Rig rig;
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

TEST(AStandardHostScenario, EveryScenarioOfTheDirectoryIsReplayedHere) {
    EXPECT_EQ(HostScenario::scenarioNames(), (std::vector<std::string>{"static_host_clock", "static_host_http",
                                                                       "static_host_kv", "static_host_notify"}));
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
