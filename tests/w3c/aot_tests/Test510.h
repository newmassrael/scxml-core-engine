#pragma once
#include "SimpleAotTest.h"
#include "test510_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor External Queue
 *
 * Tests that Basic HTTP messages are placed in the external event queue (not internal queue).
 * W3C SCXML C.1 specifies that internal queue has higher priority than external queue.
 *
 * Expected behavior:
 * - Send HTTP event via BasicHTTPEventProcessor with target="http://localhost:8080/test"
 * - SendHelper detects HTTP target and calls raiseExternal() to place event in external queue
 * - Raise internal event via <raise> action (goes to internal queue with higher priority)
 * - Process internal event first (transition to s1)
 * - Process HTTP event second (transition to pass)
 *
 * Uses Static Hybrid approach: SendHelper.isInternalTarget() detects HTTP URLs and
 * routes to external queue. W3CTestRunner provides HTTP server infrastructure automatically.
 */
struct Test510 : public SimpleAotTest<Test510, 510> {
    static constexpr const char *DESCRIPTION = "BasicHTTP external queue (W3C C.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test510::test510;
};

// Auto-register
inline static AotTestRegistrar<Test510> registrar_Test510;

}  // namespace RSM::W3C::AotTests
