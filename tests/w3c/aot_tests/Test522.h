#pragma once
#include "HttpAotTest.h"
#include "test522_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor location field
 *
 * Tests that the BasicHTTP Event I/O Processor can send messages to external
 * HTTP targets using the processor's location field (target URL).
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Accept target URLs in the <send> element's target attribute
 * - Send HTTP POST requests to the specified URL
 * - Deliver any response event back to the state machine
 *
 * Expected behavior:
 * - <send event="test" target="http://localhost:8080/test"> triggers HTTP POST
 * - W3CHttpTestServer receives POST and sends "test" event back
 * - Any event (except timeout/error) transitions to pass state
 * - Wildcard transition "*" catches the response event
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time states/transitions)
 * - HTTP target URL: Static literal "http://localhost:8080/test"
 * - Event name: Static literal "test"
 * - No JSEngine required (NEEDS_JSENGINE = false)
 * - External HTTP server handles I/O (W3CHttpTestServer)
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isInvalidTarget() shared validation logic
 * - HttpEventTarget shared HTTP POST implementation
 * - W3CHttpTestServer shared test infrastructure
 * - EventMatchingHelper for wildcard transition matching
 *
 * Key Distinction from Test 521:
 * - Test 521: Uses targetexpr (dynamic expression) → Static Hybrid with JSEngine
 * - Test 522: Uses target (static literal) → Pure Static, no JSEngine needed
 */
struct Test522 : public HttpAotTest<Test522, 522> {
    static constexpr const char *DESCRIPTION = "BasicHTTP location field (W3C C.2 AOT Pure Static)";
    using SM = RSM::Generated::test522::test522;
};

// Auto-register
inline static AotTestRegistrar<Test522> registrar_Test522;

}  // namespace RSM::W3C::AotTests
