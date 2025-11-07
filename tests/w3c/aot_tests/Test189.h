#pragma once
#include "SimpleAotTest.h"
#include "test189_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.1: Internal queue priority via target="#_internal"
 *
 * Tests that events sent with target="#_internal" are placed on the internal event queue,
 * which has higher priority than the external event queue during event processing.
 *
 * Test Scenario:
 * 1. On entering state s0, send two events:
 *    - event2 to external queue (via normal <send>)
 *    - event1 to internal queue (via <send target="#_internal">)
 * 2. Even though event2 is sent first, event1 should be processed first
 *    because internal queue has higher priority than external queue
 * 3. Transition on event1 leads to pass state
 * 4. Transition on event2 leads to fail state (should not be reached)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (datamodel="ecmascript" declared but not used)
 * - Uses Helper functions: SendHelper for internal/external queue routing
 *
 * W3C SCXML Features:
 * - W3C SCXML C.1: #_internal target for internal event queue
 * - W3C SCXML 5.9: Event processing order (internal queue > external queue)
 * - W3C SCXML 6.2: <send> element with event and target attributes
 */
struct Test189 : public SimpleAotTest<Test189, 189> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML C.1: target='#_internal' adds event to internal queue (higher priority than external queue)";
    using SM = SCE::Generated::test189::test189;
};

// Auto-register
inline static AotTestRegistrar<Test189> registrar_Test189;

}  // namespace SCE::W3C::AotTests
