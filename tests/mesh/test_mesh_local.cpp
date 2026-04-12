// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

// Verify EventQueueBridge compiles with a concrete event type
static_assert(
    sizeof(SCE::Mesh::EventQueueBridge<int, 64>) > 0,
    "EventQueueBridge must be instantiable with int");

// Verify TransportRouter template is well-formed by instantiating
// with a minimal mock engine (avoids full StaticExecutionEngine deps)
namespace {

struct MockEngine {
    enum class Event { dummy };
    void processEvent(Event) {}
};

using Router = SCE::Generated::brake::TransportRouter<MockEngine>;

static_assert(sizeof(Router) > 0,
              "TransportRouter must be instantiable");

}  // namespace

int main() {
    // Instantiate TransportRouter with mock engine
    MockEngine motor;
    Router router(motor);

    // Verify route_send compiles (runtime path not tested — Phase 2)
    router.route_send("#motor", MockEngine::Event::dummy);

    // Verify EventQueueBridge push/pop/empty compile
    SCE::Mesh::EventQueueBridge<int, 64> bridge;
    (void)bridge.try_push(42);
    (void)bridge.empty();

    std::printf("SCE Mesh compile verification: PASS\n");
    return 0;
}
