#pragma once
#include "HttpAotTest.h"
#include "test531_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: _scxmleventname Parameter Event Name Override
 *
 * Tests that when <send> has NO event attribute, the _scxmleventname parameter
 * value becomes the event name for BasicHTTP Event I/O Processor.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Use _scxmleventname parameter value as event name when event attribute is absent
 * - Send HTTP POST request with _scxmleventname in POST parameters
 * - Server extracts _scxmleventname value and uses it as the event name
 *
 * Test SCXML structure:
 * ```xml
 * <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
 *       target="http://localhost:8080/test">
 *   <param name="_scxmleventname" expr="'test'"/>
 * </send>
 * ```
 *
 * Expected behavior:
 * 1. <send> has NO event attribute (only type and target)
 * 2. <param name="_scxmleventname" expr="'test'"> specifies event name
 * 3. EventDataHelper::buildJsonFromParams() encodes as POST parameters
 * 4. W3CHttpTestServer receives POST with "_scxmleventname=test"
 * 5. Server extracts "test" from _scxmleventname param and echoes "test" event
 * 6. State machine receives "test" event and transitions to pass state
 *
 * Key W3C SCXML C.2 Feature:
 * - When event attribute is ABSENT, _scxmleventname param value becomes event name
 * - This allows dynamic event name specification via parameters
 * - Server MUST extract _scxmleventname from POST parameters and use as event name
 *
 * Difference from other HTTP tests:
 * - test 518: event="test" + namelist (event name in attribute)
 * - test 519: event="test" + param (event name in attribute)
 * - test 520: NO event + content-only (server generates HTTP.POST event)
 * - test 531: NO event + _scxmleventname param (event name from parameter)
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * ✅ Pure Static Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test"
 * - Param value: Static string literal expr="'test'" (NO variable reference)
 * - No JSEngine needed: Param value is static literal, not ECMAScript expression
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - EventDataHelper::buildJsonFromParams() shared for POST parameter encoding
 * - W3CHttpTestServer._scxmleventname handling shared (no duplication)
 * - Single Source of Truth for _scxmleventname parameter extraction
 *
 * ✅ W3C SCXML C.2 Compliance:
 * - Real HTTP POST operations (not fake/mock implementation)
 * - Actual network traffic to localhost:8080/test
 * - Server-side _scxmleventname extraction and event name override
 * - Full BasicHTTP Event I/O Processor specification support
 *
 * Pure Static Optimization Implementation:
 * The param value expr="'test'" is a static string literal in SCXML.
 * The code generator detects this at parse time using _is_static_string_literal()
 * and extracts the value "test" using _extract_static_string_literal().
 * Generated code uses direct C++ assignment: params["_scxmleventname"].push_back("test")
 * No JSEngine initialization, no sessionId_ field - pure compile-time static code.
 *
 * Performance benefit: ~0.01ms JSEngine initialization overhead eliminated.
 * Code simplicity: Generated code has zero runtime dependencies on JSEngine.
 */
struct Test531 : public HttpAotTest<Test531, 531> {
    static constexpr const char *DESCRIPTION = "BasicHTTP _scxmleventname param event name (W3C C.2 AOT Static)";
    using SM = SCE::Generated::test531::test531;
};

// Auto-register
inline static AotTestRegistrar<Test531> registrar_Test531;

}  // namespace SCE::W3C::AotTests
