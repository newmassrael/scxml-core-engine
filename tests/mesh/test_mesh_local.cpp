// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
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

#include <cstdio>
#include <cstring>
#include <functional>
#include <optional>
#include <string>
#include <utility>

// Verify EventQueueBridge compiles with a concrete event type
static_assert(sizeof(SCE::Mesh::EventQueueBridge<int, 64>) > 0, "EventQueueBridge must be instantiable with int");

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
