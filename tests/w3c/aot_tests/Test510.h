#pragma once
#include "HttpAotTest.h"
#include "test510_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor External Queue
 *
 * Tests that Basic HTTP messages are placed in the external event queue (not internal queue).
 * W3C SCXML C.1 specifies that internal queue has higher priority than external queue.
 *
 * Expected behavior:
 * - Send HTTP event via BasicHTTPEventProcessor with target="http://localhost:8080/test"
 * - StaticExecutionEngine detects HTTP target and performs real HTTP POST
 * - HTTP server responds, event placed in external queue via callback
 * - Raise internal event via <raise> action (goes to internal queue with higher priority)
 * - Process internal event first (transition to s1)
 * - Process HTTP event second (transition to pass)
 *
 * Uses Static Hybrid approach: StaticExecutionEngine.raiseExternal() detects HTTP URLs
 * and performs real HTTP POST. HttpAotTest provides HTTP server infrastructure.
 */
struct Test510 : public HttpAotTest<Test510, 510> {
    static constexpr const char *DESCRIPTION = "BasicHTTP external queue (W3C C.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test510::test510;
};

// Auto-register
inline static AotTestRegistrar<Test510> registrar_Test510;

}  // namespace SCE::W3C::AotTests
