#pragma once
#include "HttpAotTest.h"
#include "test567_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP param encoding in _event.data
 *
 * Verifies that when sending an event with <param> elements via BasicHTTP Event I/O Processor,
 * the processor correctly encodes parameters in the HTTP POST body and populates _event.data
 * with the parameter structure in the response event for ECMAScript datamodel.
 *
 * Test flow:
 * 1. State machine initializes to s0
 * 2. Entry action in s0: Initialize Var1 = 2 in ECMAScript datamodel
 * 3. Entry action in s0: Send HTTP POST to http://localhost:8080/test with <param name="param1" expr="2">
 * 4. SendHelper evaluates expr="2" with JSEngine → EventWithMetadata(data={"param1":"2"})
 * 5. StaticExecutionEngine.raiseExternal() detects BasicHTTPEventProcessor target
 * 6. HttpEventTarget sends real HTTP POST with URL-encoded body: "param1=2"
 * 7. W3CHttpTestServer receives POST, parses body, returns response with _event.data={"param1":"2"}
 * 8. HttpAotTest callback maps response event name to Event enum
 * 9. State machine processes event: SystemVariableHelper sets _event.data in JSEngine session
 * 10. Guard evaluation: JSEngine evaluates "Var1 == 2" → true
 * 11. Transition s0 → s1: Assign Var1 = _event.data.param1 (JSEngine evaluates)
 * 12. Guard evaluation: JSEngine evaluates "_event.data.param1 == 2" → true
 * 13. Transition s1 → pass (final state)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel (variable initialization, guard evaluation, _event.data access)
 * - Uses Helper functions: SendHelper (param evaluation), EventDataHelper (JSON building),
 *   SystemVariableHelper (_event.data setup), GuardHelper (guard evaluation)
 * - Zero Duplication: Reuses Interpreter's HttpEventTarget and W3CHttpTestServer
 *
 * W3C SCXML Features:
 * - BasicHTTP Event I/O Processor (W3C SCXML C.2)
 * - <param> expression evaluation with JSEngine (W3C SCXML 5.11.2)
 * - _event.data structure for event parameters (W3C SCXML 5.10)
 * - URL-encoded HTTP POST body with param values (W3C SCXML C.2)
 * - Guard conditions accessing _event.data properties (W3C SCXML 5.9)
 *
 * Infrastructure:
 * - HttpAotTest base class manages W3CHttpTestServer lifecycle
 * - StaticExecutionEngine.raiseExternal() detects HTTP targets via EventWithMetadata.originType
 * - HttpEventTarget handles real HTTP POST operations (shared with Interpreter)
 * - Async event processing: HTTP request → server response → event callback → state machine tick
 */
struct Test567 : public HttpAotTest<Test567, 567> {
    static constexpr const char *DESCRIPTION = "BasicHTTP param encoding (W3C C.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test567::test567;
};

// Auto-register
inline static AotTestRegistrar<Test567> registrar_Test567;

}  // namespace SCE::W3C::AotTests
