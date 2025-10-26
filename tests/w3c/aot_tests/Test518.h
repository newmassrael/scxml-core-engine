#pragma once
#include "HttpAotTest.h"
#include "test518_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor Namelist Encoding
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly encodes
 * namelist values as HTTP POST parameters.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Encode namelist variables as POST parameters (_scxmleventname for event name)
 * - Send HTTP POST request to target URI with application/x-www-form-urlencoded
 * - Include all namelist variables in POST body (e.g., "Var1=2&_scxmleventname=test")
 *
 * Expected behavior:
 * - State machine initializes Var1=2 in datamodel
 * - <send> with namelist="Var1" triggers HTTP POST to http://localhost:8080/test
 * - NamelistHelper::evaluateNamelist() evaluates Var1 variable via JSEngine
 * - EventDataHelper::buildJsonFromParams() builds POST parameter encoding
 * - W3CHttpTestServer receives POST with "Var1=2&_scxmleventname=test"
 * - Server validates POST parameters and sends "test" event back to state machine
 * - State machine transitions to pass state
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * This test uses Static Hybrid strategy, NOT Interpreter wrapper, because:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test" (not dynamic expression)
 * - Namelist: Static variable name "Var1" (not dynamic expression)
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 * - No engine mixing: AOT state machine + external HTTP server (W3CHttpTestServer)
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - NamelistHelper::evaluateNamelist() shared for namelist variable evaluation
 * - EventDataHelper::buildJsonFromParams() shared for POST parameter encoding
 * - Single Source of Truth for HTTP POST encoding logic
 *
 * ✅ Static Hybrid: Static structure + JSEngine for ECMAScript datamodel
 * - State machine structure is static (compile-time known)
 * - JSEngine evaluates ECMAScript datamodel (Var1=2) at runtime
 * - NamelistHelper uses JSEngine to evaluate namelist variables
 * - No dynamic expressions (targetexpr, namelistexpr) requiring Interpreter
 *
 * Key Distinction (ARCHITECTURE.md lines 274-283):
 * - ✅ Static namelist (`namelist="Var1"`) → Static/Static Hybrid compatible
 * - ✅ Static URL (`target="http://..."`) → Static/Static Hybrid compatible
 * - ❌ Dynamic expression (`namelistexpr="varName"`) → Would require Interpreter
 * - ❌ Dynamic expression (`targetexpr="urlVar"`) → Would require Interpreter
 *
 * This validates that HTTP URL targets with namelist encoding are fully
 * compatible with Static Hybrid approach when using external infrastructure
 * and shared Helper functions for namelist evaluation and POST encoding.
 */
struct Test518 : public HttpAotTest<Test518, 518> {
    static constexpr const char *DESCRIPTION = "BasicHTTP namelist encoding (W3C C.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test518::test518;
};

// Auto-register
inline static AotTestRegistrar<Test518> registrar_Test518;

}  // namespace RSM::W3C::AotTests
