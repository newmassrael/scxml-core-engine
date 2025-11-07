#pragma once
#include "SimpleAotTest.h"
#include "test190_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.10.1: External queue priority via targetexpr with sessionid
 *
 * Tests that events sent with targetexpr evaluating to #_scxml_sessionid are placed
 * on the external event queue, which has lower priority than the internal event queue.
 *
 * Test Scenario:
 * 1. Initialize datamodel:
 *    - Var1 = '#_scxml_' (string literal)
 *    - Var2 = _sessionid (runtime session ID)
 * 2. On entering state s0, execute in order:
 *    - send event2 with targetexpr="Var1" (evaluates to '#_scxml_', goes to external queue)
 *    - raise event1 (goes to internal queue)
 *    - send timeout event (external queue, after event2)
 * 3. Internal queue (event1) should be processed first
 * 4. Then external queue (event2) should be processed before timeout
 * 5. Transition on event1 in s0 leads to s1
 * 6. Transition on event2 in s1 leads to pass
 * 7. Any other event leads to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and targetexpr evaluation
 * - Uses Helper functions: SendHelper, DataModelHelper, GuardHelper
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.10.1: #_scxml_sessionid target for external event queue
 * - W3C SCXML 5.9: Event processing order (internal queue > external queue)
 * - W3C SCXML 6.2.4: targetexpr attribute for dynamic target evaluation
 * - W3C SCXML 5.9.2: <raise> element for internal queue
 * - W3C SCXML 5.2: ECMAScript datamodel with _sessionid system variable
 */
struct Test190 : public SimpleAotTest<Test190, 190> {
    static constexpr const char *DESCRIPTION = "W3C SCXML C.1: External queue via #_scxml_sessionid target";
    using SM = SCE::Generated::test190::test190;
};

// Auto-register
inline static AotTestRegistrar<Test190> registrar_Test190;

}  // namespace SCE::W3C::AotTests
