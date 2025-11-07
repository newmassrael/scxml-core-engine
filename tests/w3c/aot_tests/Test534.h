#pragma once
#include "HttpAotTest.h"
#include "test534_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor _scxmleventname parameter transmission
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly sends
 * the event name as the _scxmleventname parameter in HTTP POST requests.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Include _scxmleventname parameter in HTTP POST with event name value
 * - Send HTTP POST request with application/x-www-form-urlencoded
 * - State machine can access _scxmleventname parameter from event data
 *
 * Expected behavior:
 * - <send event="test" type="BasicHTTP" target="http://localhost:8080/test">
 * - HTTP POST includes "_scxmleventname=test" parameter
 * - W3CHttpTestServer receives POST with "_scxmleventname=test"
 * - Server validates parameter and echoes "test" event back
 * - Transition expr="_scxmleventname test" guards the transition (JSEngine evaluation)
 * - State machine transitions to pass state only if guard succeeds
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test"
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 * - No engine mixing: AOT state machine + external HTTP server (W3CHttpTestServer)
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - EventDataHelper::buildJsonFromParams() shared for POST parameter encoding
 * - Single Source of Truth for HTTP POST encoding logic
 *
 * ✅ Static Hybrid: JSEngine for transition guard evaluation
 * - State machine structure is static (compile-time known)
 * - Transition guard expr="_scxmleventname test" requires JSEngine for ECMAScript evaluation
 * - JSEngine evaluates guard condition at runtime with event data access
 * - W3C SCXML 5.9 & C.2: System-reserved identifier _scxmleventname requires JSEngine
 *
 * Key W3C SCXML Features:
 * - W3C SCXML C.2: BasicHTTP Event I/O Processor parameter encoding
 * - W3C SCXML 3.13: Transition guards with ECMAScript expressions
 * - W3C SCXML 5.9: ECMAScript datamodel for runtime expression evaluation
 *
 * This validates that:
 * 1. HTTP POST requests include _scxmleventname parameter
 * 2. State machine can access HTTP parameters via ECMAScript expressions
 * 3. Transition guards can validate event data parameters at runtime
 */
struct Test534 : public HttpAotTest<Test534, 534> {
    static constexpr const char *DESCRIPTION = "BasicHTTP _scxmleventname transmission (W3C C.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test534::test534;
};

// Auto-register
inline static AotTestRegistrar<Test534> registrar_Test534;

}  // namespace SCE::W3C::AotTests
