#pragma once
#include "HttpAotTest.h"
#include "test532_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: HTTP Method Name Fallback for Event Naming
 *
 * Tests that when <send> has NO event attribute and NO _scxmleventname parameter,
 * the SCXML Processor uses the HTTP method name (HTTP.POST) as the event name
 * for BasicHTTP Event I/O Processor.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Use HTTP method name as event name when both event attribute and _scxmleventname are absent
 * - Send HTTP POST request with content as message body
 * - Server responds with HTTP method name (HTTP.POST) as the event name
 *
 * Test SCXML structure:
 * ```xml
 * <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
 *       target="http://localhost:8080/test">
 *   <content>test</content>
 * </send>
 * ```
 *
 * Expected behavior:
 * 1. <send> has NO event attribute and NO _scxmleventname parameter
 * 2. Only <content> element with static string "test"
 * 3. StaticExecutionEngine sends HTTP POST to localhost:8080/test
 * 4. W3CHttpTestServer receives POST with "test" in message body
 * 5. Server extracts HTTP method name "POST" and echoes "HTTP.POST" event
 * 6. State machine receives "HTTP.POST" event and transitions to pass state
 *
 * Key W3C SCXML C.2 Feature:
 * - When BOTH event attribute AND _scxmleventname parameter are ABSENT, HTTP method name becomes event name
 * - This is the fallback mechanism when no explicit event name is provided
 * - Server MUST use HTTP method name (e.g., "HTTP.POST") as the event name
 *
 * Difference from other HTTP tests:
 * - test 518: event="test" + namelist (event name in attribute)
 * - test 519: event="test" + param (event name in attribute)
 * - test 520: NO event + content-only (first W3C test for HTTP method fallback)
 * - test 531: NO event + _scxmleventname param (event name from parameter)
 * - test 532: NO event + NO _scxmleventname + content (HTTP method name fallback)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * ✅ Pure Static Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test"
 * - Content: Static string literal "test" (NO variable reference)
 * - Event names: Static literals ("HTTP.POST", "*")
 * - Delay: Static delay string "3s"
 * - No JSEngine needed: All values are static literals
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - W3CHttpTestServer HTTP method name extraction shared (no duplication)
 * - Single Source of Truth for HTTP method name fallback logic
 * - EventWithMetadata originType detection shared
 *
 * ✅ W3C SCXML C.2 Compliance:
 * - Real HTTP POST operations (not fake/mock implementation)
 * - Actual network traffic to localhost:8080/test
 * - Server-side HTTP method name extraction ("POST" → "HTTP.POST")
 * - Full BasicHTTP Event I/O Processor specification support
 *
 * Pure Static Implementation:
 * All content is static literals, no ECMAScript expressions.
 * Code generator produces pure static code with Event::Empty for content-only send.
 * Generated code uses Event::HTTP_POST enum for the expected response event.
 * No JSEngine initialization, no sessionId_ field - pure compile-time static code.
 *
 * Performance benefit: Zero runtime overhead, compile-time event matching.
 * Code simplicity: Generated code has zero runtime dependencies on JSEngine.
 */
struct Test532 : public HttpAotTest<Test532, 532> {
    static constexpr const char *DESCRIPTION = "BasicHTTP HTTP method name fallback (W3C C.2 AOT Static)";
    using SM = SCE::Generated::test532::test532;
};

// Auto-register
inline static AotTestRegistrar<Test532> registrar_Test532;

}  // namespace SCE::W3C::AotTests
