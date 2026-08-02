// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE-VERIFIES: mesh-3.1
//
// SCE Mesh local_transport compile verification test.
//
// Validates that generated mesh transport code compiles against
// generated state machine headers. No runtime assertions —
// compilation success IS the test (runtime behavior is covered by
// test_mesh_local_runtime.cpp).

#include "brake_sm.h"
#include "brake_transport.h"
#include "mesh/EventQueueBridge.h"
#include "mesh/SchedulerConcepts.h"
#include "motor_sm.h"

#include <chrono>
#include <cstdio>
#include <cstring>
#include <functional>
#include <optional>
#include <string>
#include <utility>

// Verify EventQueueBridge compiles with a concrete event type
static_assert(sizeof(SCE::Mesh::EventQueueBridge<int, 64>) > 0, "EventQueueBridge must be instantiable with int");

// SCE_MESH.md §3.1 publishes TickScheduling / EventDrivenScheduling as a
// contract for the integrator's own scheduler — SCE ships no scheduler and
// calls none, so nothing else in the tree would notice a drift in either
// predicate. Both directions are pinned: a conforming shape must satisfy
// it, and a shape missing one required member must not. Without the
// negative half, a predicate that accidentally accepted everything would
// still pass.
namespace {

/// Minimal conforming tick scheduler: names its Duration, ticks, reports
/// a deadline.
struct ModelTickScheduler {
    using Duration = std::chrono::milliseconds;

    void tick() {}

    Duration deadline() const {
        return Duration{0};
    }
};

/// Minimal conforming event-driven scheduler.
struct ModelEventScheduler {
    using Event = int;

    void onEvent(Event) {}
};

/// Conforming except for `deadline()` — must NOT satisfy TickScheduling.
struct TickSchedulerMissingDeadline {
    using Duration = std::chrono::milliseconds;

    void tick() {}
};

}  // namespace

#if __cpp_concepts >= 202002L
static_assert(SCE::Mesh::TickScheduling<ModelTickScheduler>,
              "a tick+deadline+Duration shape must satisfy TickScheduling");
static_assert(!SCE::Mesh::TickScheduling<TickSchedulerMissingDeadline>,
              "TickScheduling must reject a scheduler with no deadline()");
static_assert(SCE::Mesh::EventDrivenScheduling<ModelEventScheduler>,
              "an onEvent+Event shape must satisfy EventDrivenScheduling");
static_assert(!SCE::Mesh::EventDrivenScheduling<ModelTickScheduler>,
              "EventDrivenScheduling must reject a tick-only scheduler");
#endif

// The C++17 fallback exposes the same three predicates as constexpr bool,
// so `if constexpr` capability detection keeps working without concepts.
static_assert(SCE::Mesh::HasTick<ModelTickScheduler>, "HasTick must detect tick()");
static_assert(SCE::Mesh::HasDeadline<ModelTickScheduler>, "HasDeadline must detect deadline()");
static_assert(!SCE::Mesh::HasOnEvent<ModelTickScheduler>, "HasOnEvent must not fire on a tick-only scheduler");
static_assert(SCE::Mesh::HasOnEvent<ModelEventScheduler>, "HasOnEvent must detect onEvent()");
static_assert(!SCE::Mesh::HasDeadline<TickSchedulerMissingDeadline>,
              "HasDeadline must not fire when deadline() is absent");

// Verify TransportRouter template is well-formed by instantiating
// with a minimal mock engine that provides the getPolicy() API
// required by the unified per-target dispatch template.
namespace {

struct MockEngine {
    enum class Event { dummy };

    struct Policy {
        static std::optional<Event> getEventFromName(const char *name) {
            if (std::strcmp(name, "dummy") == 0) {
                return Event::dummy;
            }
            return std::nullopt;
        }
    };

    // SCE::Core::AotSmMeshIntegration concept requires the engine type
    // to expose `PolicyType` as a nested typedef alias for the policy
    // class. The AOT-emitted SM class uses the same name; the mock
    // mirrors it so the static_assert in TransportRouter passes against
    // the same contract production code is checked against.
    using PolicyType = Policy;

    // Mirror the production engine's metadata surface so MeshDispatch routes
    // this mock through the same raiseExternal(EventWithMetadata) overload —
    // a simpler shape would force dispatchEnvelope into a feature-detected
    // fallback and let test paths silently diverge from production.
    struct EventWithMetadata {
        Event event{Event::dummy};
        std::string data;
        std::string invokeId;
        std::string type;
        std::string originType;
        std::string origin;
        std::string sendId;
    };

    Policy policy_;

    Policy &getPolicy() {
        return policy_;
    }

    std::string currentEventInvokeId() const {
        return {};
    }

    void processEvent(Event) {}

    void raiseExternal(Event, const std::string & = {}) {}

    void raiseExternal(const EventWithMetadata &) {}

    // TransportRouter's ctor installs the outbound dispatch hook on
    // every hosted session via this method; the mock just stores the
    // callback (ignored by this compile test).
    using MeshSendCb = std::function<bool(const std::string &, const std::string &, const std::string &,
                                          const std::string &, const std::string &)>;
    MeshSendCb mesh_send_cb_;

    void setMeshSendCallback(MeshSendCb cb) {
        mesh_send_cb_ = std::move(cb);
    }
};

// Session-first ctor injection: SenderEngine template param precedes
// per-target engine params, and the session pointer array is passed
// first to the ctor. The session's MockEngine is the same type as the
// local target engine here; production code uses different engine types.
using Router = SCE::Generated::brake::TransportRouter<MockEngine, MockEngine>;

static_assert(sizeof(Router) > 0, "TransportRouter must be instantiable");

}  // namespace

int main() {
    MockEngine sender;
    MockEngine motor;
    Router router({&sender}, motor);

    // Verify route_send compiles with MeshEnvelope API
    SCE::Mesh::MeshEnvelope env;
    env.type = "dummy";
    (void)router.route_send("#motor", env);

    // Verify EventQueueBridge push/pop/empty compile
    SCE::Mesh::EventQueueBridge<int, 64> bridge;
    (void)bridge.try_push(42);
    (void)bridge.empty();

    std::printf("SCE Mesh compile verification: PASS\n");
    return 0;
}
