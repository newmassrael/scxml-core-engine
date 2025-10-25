#pragma once
#include "SimpleAotTest.h"
#include "test513_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor Success Response
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor responds with
 * HTTP 200 OK when receiving well-formed events.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Accept HTTP POST requests at the access URI
 * - Respond with 2XX success code for well-formed events
 * - Place received events in the external event queue
 *
 * Expected behavior:
 * - W3CTestRunner starts HTTP server infrastructure automatically
 * - State machine sends HTTP POST to http://localhost:8080/test via <send>
 * - BasicHTTP Event I/O Processor receives request and responds with 200 OK
 * - Event is placed in external queue and state machine transitions to pass
 *
 * Original W3C test is manual (requires external wget/curl to send HTTP POST).
 * This implementation automates validation by having the state machine send
 * HTTP events to itself, validating successful 200 OK response reception.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * This test uses Static Hybrid strategy, NOT Interpreter wrapper, because:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test" (not dynamic expression)
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 * - No engine mixing: AOT state machine + external HTTP server (W3CHttpTestServer)
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - Single Source of Truth for HTTP URL detection logic
 * - External HTTP infrastructure (W3CHttpTestServer) is separate, not duplicated
 *
 * Key Distinction (ARCHITECTURE.md lines 274-283):
 * - ✅ Static URL (`target="http://..."`) → Static/Static Hybrid compatible
 * - ❌ Dynamic expression (`targetexpr="urlVar"`) → Would require Interpreter
 *
 * This validates that HTTP URL targets with compile-time known values
 * are fully compatible with Static Hybrid approach when using external
 * infrastructure (not implementing the processor itself).
 */
struct Test513 : public SimpleAotTest<Test513, 513> {
    static constexpr const char *DESCRIPTION = "BasicHTTP success response (W3C C.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test513::test513;
};

// Auto-register
inline static AotTestRegistrar<Test513> registrar_Test513;

}  // namespace RSM::W3C::AotTests
