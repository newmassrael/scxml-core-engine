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
#include <optional>

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

    Policy policy_;
    Policy& getPolicy() { return policy_; }
    void processEvent(Event) {}
    void raiseExternal(Event, const std::string& = {}) {}
};

using Router = SCE::Generated::brake::TransportRouter<MockEngine>;

static_assert(sizeof(Router) > 0,
              "TransportRouter must be instantiable");

}  // namespace

int main() {
    // Instantiate TransportRouter with mock engine
    MockEngine motor;
    Router router(motor);

    // Verify route_send compiles with MeshSendRequest API
    SCE::Mesh::MeshSendRequest req;
    req.target = "#motor";
    req.eventName = "dummy";
    (void)router.route_send(req);

    // Verify EventQueueBridge push/pop/empty compile
    SCE::Mesh::EventQueueBridge<int, 64> bridge;
    (void)bridge.try_push(42);
    (void)bridge.empty();

    std::printf("SCE Mesh compile verification: PASS\n");
    return 0;
}
