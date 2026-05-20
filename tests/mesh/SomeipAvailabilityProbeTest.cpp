// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh — axis-6 A6-002 SomeipAvailabilityProbe unit tests.
//
// Pins the defensive-idempotent absorption contract:
//   1. Handler is invoked at least once even if the mock vsomeip
//      application does NOT fire the initial-edge callback (the entire
//      reason A6-002 exists).
//   2. Handler is invoked at most twice when the mock vsomeip DOES
//      fire its own initial callback (idempotency contract — caller's
//      handler must tolerate the duplicate).
//   3. The synthesized invocation carries the application's current
//      `is_available(service, instance)` state at the moment of
//      `probeAndDispatch`.
//
// The mock is duck-typed (no `vsomeip::application` inheritance) — the
// probe is templated on the application type so a minimal mock with
// only the two methods the probe touches is sufficient.

#include "mesh/third_party/SomeipAvailabilityProbe.h"

#include <gtest/gtest.h>

// `vsomeip.hpp` brings the `namespace vsomeip = vsomeip_v3;` alias that
// the probe and the rest of SCE write in unqualified form. The lower-
// level handler.hpp / primitive_types.hpp headers only declare the
// types under `vsomeip_v3` without the alias, so including just those
// would leave `vsomeip::availability_handler_t` unresolved.
#include <vsomeip/vsomeip.hpp>

#include <memory>
#include <tuple>
#include <vector>

namespace {

class MockApp {
public:
    bool fire_initial_callback = false;
    bool current_available = false;
    std::vector<std::tuple<vsomeip::service_t, vsomeip::instance_t, bool>>
        handler_invocations;

    // Duck-typed surface: only the two methods the probe calls.
    void register_availability_handler(
        vsomeip::service_t service, vsomeip::instance_t instance,
        const vsomeip::availability_handler_t& handler) {
        if (fire_initial_callback) {
            handler(service, instance, current_available);
        }
    }

    bool is_available(vsomeip::service_t, vsomeip::instance_t) const {
        return current_available;
    }

    vsomeip::availability_handler_t make_capturing_handler() {
        return [this](vsomeip::service_t s, vsomeip::instance_t i,
                      bool avail) {
            handler_invocations.emplace_back(s, i, avail);
        };
    }
};

constexpr vsomeip::service_t kTestService = 0x1234;
constexpr vsomeip::instance_t kTestInstance = 0x5678;

}  // namespace

TEST(SomeipAvailabilityProbeTest,
     HandlerInvokedExactlyOnceWhenVsomeipSkipsInitialCallback) {
    auto app = std::make_shared<MockApp>();
    app->fire_initial_callback = false;
    app->current_available = true;

    SCE::Mesh::ThirdParty::probeAndDispatch(
        app, kTestService, kTestInstance, app->make_capturing_handler());

    ASSERT_EQ(app->handler_invocations.size(), 1u)
        << "probe path must invoke handler exactly once when vsomeip "
           "skips the initial-edge callback";
    const auto& [svc, inst, avail] = app->handler_invocations[0];
    EXPECT_EQ(svc, kTestService);
    EXPECT_EQ(inst, kTestInstance);
    EXPECT_TRUE(avail) << "synthesized invocation must carry the current "
                         "is_available() state";
}

TEST(SomeipAvailabilityProbeTest,
     HandlerInvokedTwiceWhenVsomeipFiresInitialCallback) {
    auto app = std::make_shared<MockApp>();
    app->fire_initial_callback = true;
    app->current_available = false;

    SCE::Mesh::ThirdParty::probeAndDispatch(
        app, kTestService, kTestInstance, app->make_capturing_handler());

    ASSERT_EQ(app->handler_invocations.size(), 2u)
        << "callback path + probe path produce two invocations; "
           "caller's handler must be idempotent";
    for (const auto& [svc, inst, avail] : app->handler_invocations) {
        EXPECT_EQ(svc, kTestService);
        EXPECT_EQ(inst, kTestInstance);
        EXPECT_FALSE(avail);
    }
}

TEST(SomeipAvailabilityProbeTest,
     SynthesizedInvocationCarriesCurrentIsAvailableValue) {
    auto app = std::make_shared<MockApp>();
    app->fire_initial_callback = false;
    app->current_available = false;
    bool observed = true;
    SCE::Mesh::ThirdParty::probeAndDispatch(
        app, kTestService, kTestInstance,
        [&observed](vsomeip::service_t /*svc*/, vsomeip::instance_t /*inst*/,
                    bool a) { observed = a; });
    EXPECT_FALSE(observed);

    app->current_available = true;
    observed = false;
    SCE::Mesh::ThirdParty::probeAndDispatch(
        app, kTestService, kTestInstance,
        [&observed](vsomeip::service_t /*svc*/, vsomeip::instance_t /*inst*/,
                    bool a) { observed = a; });
    EXPECT_TRUE(observed);
}
