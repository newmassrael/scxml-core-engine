// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The replay of a scenario of `tests/host_features/scenarios` (docs/SCE_HOST_FEATURES.md),
// shared by the two C++ engines that run one: the generated machine
// (AStandardHostScenarioTest) and the Interpreter running the lowered document
// (AStaticDatamodelRunsLoweredUnderTheInterpreterTest; docs/adr/0084, D7). Both
// register the reference host on the machine the same way (`SCE::StandardHost::serve`),
// and both read a scenario through this file, so the two are held to one reading of it:
// what a `host` directive sets, what `host_invokes` and `host_state` are, and the order
// a step is taken in. What differs is only how each engine is driven and read, which it
// hands in as a `HostScenarioDriver`.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and what it
// holds afterwards. The expected values are derived from the specification, not observed
// from the host.

#pragma once

#include "host/StandardHost.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the host feature scenarios are read from it"
#endif

namespace SCE::Tests::HostScenario {

using json = nlohmann::json;
namespace host = SCE::StandardHost;

/// The directory every engine's replay reads the scenarios from.
inline std::filesystem::path scenarioDirectory() {
    return std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "scenarios";
}

/// The scenario of @p machine, `<machine>.json`.
inline json readScenario(const std::string &machine) {
    const auto path = scenarioDirectory() / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.good()) << "the scenario is not readable: " << path;
    return json::parse(in);
}

/// The stem of every scenario of the directory, sorted.
inline std::vector<std::string> scenarioNames() {
    std::vector<std::string> names;
    for (const auto &entry : std::filesystem::directory_iterator(scenarioDirectory())) {
        if (entry.path().extension() == ".json") {
            names.push_back(entry.path().stem().string());
        }
    }
    std::sort(names.begin(), names.end());
    return names;
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

/// The reference host, over a clock and a transport a scenario sets.
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

/// How one engine is driven through a scenario and read back. Each member is asked in
/// the order a step is taken: the host's directives first, then `advance` for a step's
/// `advance_ms`, then `raise` for its `event`, and then the expectations.
struct HostScenarioDriver {
    /// Move the machine's time on by the milliseconds named, and run what fell due.
    std::function<void(uint64_t)> advance;
    /// Deliver the external event named, with the JSON text of the step's `data` (empty
    /// for none), and run it to completion. `where` names the step for a failure.
    std::function<void(const std::string &where, const std::string &event, const std::string &data)> raise;
    /// The machine's current state, by its id.
    std::function<std::string()> state;
    /// Hold the variable named to the value the step expects.
    std::function<void(const std::string &where, const std::string &name, const json &want)> expectVariable;
};

/// Replay @p scenario on the machine @p driver drives, served by @p rig's host, which the
/// caller registered on the machine (`SCE::StandardHost::serve`) and started.
inline void replay(const json &scenario, Rig &rig, const HostScenarioDriver &driver) {
    ASSERT_FALSE(scenario.at("steps").empty()) << "a scenario with no steps judges nothing";
    int n = 0;
    for (const auto &step : scenario.at("steps")) {
        const std::string where = "step " + std::to_string(n++) + " (" + step.value("note", "") + ")";
        SCOPED_TRACE(where);
        if (step.contains("host")) {
            rig.directives(step.at("host"));
        }
        if (step.contains("advance_ms")) {
            driver.advance(step.at("advance_ms").get<uint64_t>());
        }
        if (step.contains("event")) {
            driver.raise(where, step.at("event").get<std::string>(),
                         step.contains("data") ? step.at("data").dump() : "");
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
            EXPECT_EQ(driver.state(), expect.at("state").get<std::string>()) << where << ": the current state";
        }
        if (expect.contains("variables")) {
            for (const auto &[variable, want] : expect.at("variables").items()) {
                driver.expectVariable(where, variable, want);
            }
        }
    }
}

}  // namespace SCE::Tests::HostScenario
