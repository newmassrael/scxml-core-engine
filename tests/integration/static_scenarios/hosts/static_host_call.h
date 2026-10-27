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

namespace SCE::Tests::StaticScenario {

template <> struct ScenarioHosts<::SCE::Generated::static_host_call::static_host_call> {
    using Machine = ::SCE::Generated::static_host_call::static_host_call;

    struct Host : ::SCE::Generated::static_host_call::StaticHostCallActions {
        json calls = json::array();

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back({{"action", "showAttempts"}, {"args", json::array({count, exhausted})}});
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
