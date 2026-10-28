// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host `static_host_call` is replayed with: a native host action takes the machine's
// variables as typed arguments, each read when the call is made, and this host records
// them as a scenario's `host_calls` writes them.

#pragma once

#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_host_call_sm.h"
#include <cstdint>
#include <optional>

namespace SCE::Tests::StaticScenario {

template <> struct ScenarioHosts<::SCE::Generated::static_host_call::static_host_call> {
    using Machine = ::SCE::Generated::static_host_call::static_host_call;

    struct Host : ::SCE::Generated::static_host_call::StaticHostCallActions {
        json calls = json::array();

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back({{"action", "showAttempts"}, {"args", json::array({count, exhausted})}});
        }

        // An argument that may be absent is a `std::optional`, which a scenario states as `null` when it is
        // absent (docs/adr/0055, step 10).
        void showDue(std::optional<int32_t> day, std::optional<bool> urgent) override {
            const json dayValue = day.has_value() ? json(*day) : json(nullptr);
            const json urgentValue = urgent.has_value() ? json(*urgent) : json(nullptr);
            calls.push_back({{"action", "showDue"}, {"args", json::array({dayValue, urgentValue})}});
        }
    };

    std::unique_ptr<Machine> make() {
        return std::make_unique<Machine>(host);
    }

    std::optional<json> hostCalls() {
        return std::exchange(host.calls, json::array());
    }

    std::optional<json> childHosts() {
        return std::nullopt;
    }

    Host host;
};

}  // namespace SCE::Tests::StaticScenario
