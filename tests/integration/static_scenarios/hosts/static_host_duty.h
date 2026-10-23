// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host of `static_host_duty`, a machine that states a duty (docs/adr/0038): it records
// what the machine asked of it, by the name the document gives the action, and says what
// it does with the duties. The scenario replays the machine on a host that only records,
// so it only simulates the duties, and the caller that built it for a recorder says so by
// name: the machine is built through the opt-in. `AStaticHostDutyTest.cpp` holds the
// machine to the hosts that perform the duties and to the refusal of one that simulates
// without the opt-in.

#pragma once

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "runtime/SimulatedHost.h"
#include "static_host_duty_sm.h"

namespace SCE::Tests::StaticScenario {

struct DutyHost : ::SCE::Generated::static_host_duty::StaticHostDutyActions {
    explicit DutyHost(::SCE::ActionRealization declared) : declared(declared) {}

    ::SCE::ActionRealization realization() const override {
        return declared;
    }

    void startKeepalive() override {
        call("start_keepalive");
    }

    void noteEngaged() override {
        call("note_engaged");
    }

    void stopKeepalive() override {
        call("stop_keepalive");
    }

    /// The calls made since this was last called, and forgets them.
    json take() {
        return std::exchange(calls, json::array());
    }

    ::SCE::ActionRealization declared;
    json calls = json::array();

private:
    void call(const std::string &action) {
        calls.push_back({{"action", action}, {"args", json::array()}});
    }
};

template <> struct ScenarioHosts<::SCE::Generated::static_host_duty::static_host_duty> {
    using Machine = ::SCE::Generated::static_host_duty::static_host_duty;

    std::unique_ptr<Machine> make() {
        return std::make_unique<Machine>(host, ::SCE::SimulatedHost::Accept);
    }

    std::optional<json> hostCalls() {
        return host.take();
    }

    std::optional<json> childHosts() {
        return std::nullopt;
    }

    DutyHost host{::SCE::ActionRealization::Simulates};
};

}  // namespace SCE::Tests::StaticScenario
