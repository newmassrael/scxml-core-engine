// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Phase 1 compile verification test.
//
// Validates that generated mesh transport code compiles against
// generated state machine headers. No runtime assertions —
// compilation success IS the test (Phase 2 adds runtime behavior).

#include "brake_sm.h"
#include "motor_sm.h"
#include "brake_transport.h"
#include "mesh/EventQueueBridge.h"
#include "mesh/SchedulerConcepts.h"

#include <cstdio>
#include <cstring>
#include <functional>
#include <optional>
#include <string>
#include <utility>

// Verify EventQueueBridge compiles with a concrete event type
static_assert(
    sizeof(SCE::Mesh::EventQueueBridge<int, 64>) > 0,
    "EventQueueBridge must be instantiable with int");

// Verify TransportRouter template is well-formed by instantiating
// with a minimal mock engine that provides the getPolicy() API
// required by the unified per-target dispatch template.
namespace {

struct MockEngine {
    enum class Event { dummy };

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            if (std::strcmp(name, "dummy") == 0) return Event::dummy;
            return std::nullopt;
        }
    };

    // Mirror the production engine's metadata surface so MeshDispatch routes
    // this mock through the same raiseExternal(EventWithMetadata) overload —
    // a simpler shape would force dispatchEnvelope into a feature-detected
    // fallback and let test paths silently diverge from production.
    struct EventWithMetadata {
        Event event{Event::dummy};
        std::string data;
        std::string invokeId;
    };

    Policy policy_;
    Policy& getPolicy() { return policy_; }
    std::string currentEventInvokeId() const { return {}; }
    void processEvent(Event) {}
    void raiseExternal(Event, const std::string& = {}) {}
    void raiseExternal(const EventWithMetadata&) {}
    // TransportRouter's ctor installs the outbound dispatch hook on the
    // sender engine via this method; the mock just stores the callback
    // (ignored by this compile test).
    using MeshSendCb = std::function<bool(const std::string&, const std::string&,
                                          const std::string&, const std::string&,
                                          const std::string&)>;
    MeshSendCb mesh_send_cb_;
    void setMeshSendCallback(MeshSendCb cb) { mesh_send_cb_ = std::move(cb); }
};

// Sender-first ctor injection: SenderEngine template param precedes
// per-target engine params, and the sender reference is passed first
// to the ctor. The sender's MockEngine is the same type as the local
// target engine here; production code uses different engine types.
using Router = SCE::Generated::brake::TransportRouter<MockEngine, MockEngine>;

static_assert(sizeof(Router) > 0,
              "TransportRouter must be instantiable");

}  // namespace

int main() {
    MockEngine sender;
    MockEngine motor;
    Router router(sender, motor);

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
