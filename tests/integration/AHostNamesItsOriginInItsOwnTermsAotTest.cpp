// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
// it delivers in its own terms — an ending of a host-run invocation whose
// origin is {x-sce-host, motor} reads back `motor` and `x-sce-host` — C++ AOT.
// The address has no scheme, so it is no SCXML location, and the type is the
// host's: the engine neither wraps the one nor defaults the other.
//
// Fixture:
// integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_host_names_its_origin_in_its_own_terms ... HOST_INVOKER x-sce-host)`.

#include "a_host_names_its_origin_in_its_own_terms_sm.h"
#include "core/EventOrigin.h"
#include "core/HostProcessor.h"
#include "scripting/ScriptEngineProvider.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_host_names_its_origin_in_its_own_terms::a_host_names_its_origin_in_its_own_terms;

constexpr const char *HOST_TYPE = "x-sce-host";

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AHostNamesItsOriginInItsOwnTermsAotTest, AHostNamesItsOriginInItsOwnTerms) {
    SM sm;
    // An invoker whose work outlives the start: the test ends it, naming the
    // party it stands for.
    std::vector<uint64_t> tokens;
    sm.registerInvoker(HOST_TYPE, [&tokens](const ::SCE::HostInvokeEvent &event) {
        if (event.start.has_value()) {
            tokens.push_back(event.start->token);
        }
        return std::optional<::SCE::HostInvokeResponse>();
    });
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    EXPECT_EQ(sm.initialize(), std::nullopt);
    sm.step();
    ASSERT_EQ(tokens.size(), 1u) << "the host was not asked to run `job` once";

    EXPECT_TRUE(sm.completeHostInvoke(HOST_TYPE, "job", tokens[0], "{}", ::SCE::Core::EventOrigin(HOST_TYPE, "motor")))
        << "the running invocation's completion was refused";
    sm.step();

    EXPECT_EQ(sm.terminalState(), SM::State::Done) << "the run must end in `done`";
    EXPECT_EQ(sm.hostNamed(), std::optional<int64_t>(1))
        << "hostNamed = " << show(sm.hostNamed())
        << ": the completion did not read back the address and the type the host named";
}

}  // namespace SCE::Tests
