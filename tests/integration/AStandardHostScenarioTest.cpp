// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated C++ machine
// sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `SCE::StandardHost::ReferenceHost` registered as the invoker of every standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and what it
// holds afterwards. The expected values are derived from the specification, not observed
// from the host. The machines are generated from the fixtures by `tests/CMakeLists.txt`.

#include "common/SceClock.h"
#include "host/HttplibTransport.h"
#include "host/StandardHost.h"
#include "static_host_clock_sm.h"
#include "static_host_http_sm.h"
#include "static_host_kv_sm.h"
#include "static_host_notify_sm.h"

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

json readScenario(const std::string &machine) {
    const auto path =
        std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "scenarios" / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.good()) << "the scenario is not readable: " << path;
    return json::parse(in);
}

/// Answers what a scenario says and keeps what it was given.
class ScriptTransport : public host::Transport {
public:
    bool set = false;
    bool hold = false;
    bool fail = false;
    host::HttpResponse response;
    std::vector<host::HttpRequest> log;

    host::Exchange exchange(const host::HttpRequest &request) override {
        log.push_back(request);
        if (hold) {
            return {host::Exchange::Kind::Hold, {}};
        }
        if (fail || !set) {
            return {host::Exchange::Kind::Failure, {}};
        }
        return {host::Exchange::Kind::Response, response};
    }

    void directive(const json &d) {
        set = true;
        hold = d.value("hold", false);
        fail = d.contains("fail");
        response = {};
        if (hold || fail) {
            return;
        }
        response.status = d.at("status").get<int>();
        for (const auto &pair : d.at("headers")) {
            response.headers.push_back({pair[0].get<std::string>(), pair[1].get<std::string>()});
        }
        if (d.contains("body")) {
            response.body = d.at("body").get<std::string>();
        } else if (d.contains("body_hex")) {
            const std::string hex = d.at("body_hex").get<std::string>();
            for (size_t i = 0; i + 1 < hex.size(); i += 2) {
                response.body.push_back(static_cast<char>(std::stoi(hex.substr(i, 2), nullptr, 16)));
            }
        } else if (d.contains("body_repeat")) {
            const auto &repeat = d.at("body_repeat");
            for (int i = 0; i < repeat[1].get<int>(); ++i) {
                response.body += repeat[0].get<std::string>();
            }
        }
    }
};

struct Rig {
    host::ScriptedClock clock;
    ScriptTransport transport;
    host::ReferenceHost host{clock, transport};

    void directives(const json &directives) {
        for (const auto &[name, value] : directives.items()) {
            if (name == "clock") {
                host::Reading reading = *clock.reading;
                host::Zone zone = *clock.zoneNow;
                reading.utcMs = value.value("utc_ms", reading.utcMs);
                reading.monotonicMs = value.value("monotonic_ms", reading.monotonicMs);
                zone.id = value.value("zone_id", zone.id);
                zone.utcOffsetS = value.value("utc_offset_s", zone.utcOffsetS);
                clock.reading = reading;
                clock.zoneNow = zone;
            } else if (name == "permission") {
                host.setPermission(value.get<std::string>());
            } else if (name == "user_decision") {
                host.setUserDecision(value.get<std::string>());
            } else if (name == "quota") {
                host.setQuota(value.get<size_t>());
            } else if (name == "fail") {
                host.setFaults(value.get<std::map<std::string, std::string>>());
            } else if (name == "http") {
                transport.directive(value);
            } else {
                ADD_FAILURE() << "`" << name << "` is not a host directive";
            }
        }
    }

    /// What the host was started with since the last step, as a scenario writes it: the request as its
    /// record, `at_ms` being the one integer field.
    json starts() {
        json out = json::array();
        for (const auto &start : host.takeStarts()) {
            json request = json::object();
            for (const auto &[name, text] : start.params) {
                request[name] = name == "at_ms" ? json(std::stoll(text)) : json(text);
            }
            out.push_back({{"type", start.operation}, {"id", start.id}, {"request", request}});
        }
        return out;
    }

    json hostState(const json &wanted) {
        json state = json::object();
        for (const auto &[name, ignored] : wanted.items()) {
            if (name == "store") {
                json store = json::object();
                for (const auto &[key, value] : host.storeEntries()) {
                    store[key] = value;
                }
                state[name] = store;
            } else if (name == "waiting") {
                state[name] = host.waiting();
            } else if (name == "shown") {
                state[name] = host.shown();
            } else if (name == "cancelled") {
                state[name] = host.cancelled();
            } else if (name == "http") {
                json log = json::array();
                for (const auto &r : transport.log) {
                    json headers = json::array();
                    for (const auto &h : r.headers) {
                        headers.push_back({{"name", h.name}, {"value", h.value}});
                    }
                    log.push_back({{"method", r.method}, {"url", r.url}, {"headers", headers}, {"body", r.body}});
                }
                state[name] = log;
            } else {
                ADD_FAILURE() << "`" << name << "` is not host state";
            }
        }
        return state;
    }
};

constexpr int kSettleTicks = 5;

template <typename Machine>
void replay(const std::string &name, std::map<std::string, std::function<json(const Machine &)>> variables) {
    const json scenario = readScenario(name);
    ASSERT_EQ(scenario.at("machine").get<std::string>(), name);
    Rig rig;
    Machine machine;
    host::serve(machine, rig.host);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.initialize();
    const auto settle = [&machine]() {
        if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
            for (int round = 0; round < kSettleTicks; ++round) {
                machine.tick();
            }
        } else {
            machine.step();
        }
    };
    settle();
    ASSERT_FALSE(scenario.at("steps").empty()) << "a scenario with no steps judges nothing";
    int n = 0;
    for (const auto &step : scenario.at("steps")) {
        const std::string where = "step " + std::to_string(n++) + " (" + step.value("note", "") + ")";
        if (step.contains("host")) {
            rig.directives(step.at("host"));
        }
        if (step.contains("advance_ms")) {
            machine.advanceTimeMs(step.at("advance_ms").get<uint64_t>());
            settle();
        }
        if (step.contains("event")) {
            const std::string event = step.at("event").get<std::string>();
            ASSERT_TRUE(machine.resolveEventByName(event).has_value())
                << where << ": the machine's events do not match " << event;
            machine.raiseExternal(event, step.contains("data") ? step.at("data").dump() : "");
            settle();
        }
        const json &expect = step.at("expect");
        const json started = rig.starts();
        if (expect.contains("host_invokes")) {
            EXPECT_EQ(started, expect.at("host_invokes")) << where << ": what the host was asked";
        }
        if (expect.contains("host_state")) {
            EXPECT_EQ(rig.hostState(expect.at("host_state")), expect.at("host_state"))
                << where << ": what the host holds";
        }
        if (expect.contains("state")) {
            EXPECT_EQ(Machine::PolicyType::getStateName(machine.getCurrentState()),
                      expect.at("state").get<std::string>())
                << where << ": the current state";
        }
        if (expect.contains("variables")) {
            for (const auto &[variable, want] : expect.at("variables").items()) {
                const auto read = variables.find(variable);
                ASSERT_NE(read, variables.end()) << where << ": the driver reads no variable " << variable;
                EXPECT_EQ(read->second(machine), want) << where << ": variable " << variable;
            }
        }
    }
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
    std::vector<std::string> names;
    for (const auto &entry : std::filesystem::directory_iterator(std::filesystem::path(SCE_PROJECT_ROOT) / "tests" /
                                                                 "host_features" / "scenarios")) {
        if (entry.path().extension() == ".json") {
            names.push_back(entry.path().stem().string());
        }
    }
    std::sort(names.begin(), names.end());
    EXPECT_EQ(names, (std::vector<std::string>{"static_host_clock", "static_host_http", "static_host_kv",
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
