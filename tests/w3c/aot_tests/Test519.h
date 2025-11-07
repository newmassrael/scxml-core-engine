#pragma once
#include "HttpAotTest.h"
#include "test519_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor Param Encoding
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly encodes
 * <param> values as HTTP POST parameters.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Map param names and values to HTTP POST parameters
 * - Send HTTP POST request with application/x-www-form-urlencoded
 * - Include all param elements in POST body (e.g., "param1=1&_scxmleventname=test")
 *
 * Expected behavior:
 * - <send> with <param name="param1" expr="1"> triggers HTTP POST
 * - EventDataHelper::buildJsonFromParams() encodes params as POST parameters
 * - W3CHttpTestServer receives POST with "param1=1&_scxmleventname=test"
 * - Server validates POST parameters and echoes "test" event back
 * - State machine transitions to pass state
 *
 * Difference from test 518 (namelist):
 * - test 518: Uses namelist="Var1" (evaluates datamodel variables)
 * - test 519: Uses <param name="param1" expr="1"> (inline parameter values)
 * - Both tests validate HTTP POST parameter encoding (W3C SCXML C.2)
 *
 * ARCHITECTURE.md Compliance - Static Approach:
 * This test uses Static strategy (NOT Static Hybrid), because:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time known states/transitions)
 * - HTTP target URL: Static string "http://localhost:8080/test" (not dynamic expression)
 * - Param values: Static expressions (expr="1") without ECMAScript datamodel
 * - SendHelper.isHttpTarget(): Detects HTTP URL and routes to external queue
 * - No engine mixing: AOT state machine + external HTTP server (W3CHttpTestServer)
 *
 * ✅ Zero Duplication Principle:
 * - SendHelper.isHttpTarget() shared between Interpreter and AOT engines
 * - EventDataHelper::buildJsonFromParams() shared for POST parameter encoding
 * - Single Source of Truth for HTTP POST encoding logic
 *
 * ✅ Pure Static: No JSEngine required
 * - No ECMAScript datamodel (datamodel="null" from TXML conversion)
 * - Param expr="1" is static literal value, not variable reference
 * - State machine structure is static (compile-time known)
 * - No dynamic expressions (targetexpr, paramexpr) requiring Interpreter
 *
 * Key Distinction from test 518:
 * - test 518: Needs JSEngine (ECMAScript datamodel for namelist variable evaluation)
 * - test 519: Pure static (param values are literals, no datamodel variables)
 *
 * This validates that HTTP URL targets with param encoding are fully
 * compatible with Pure Static approach when parameter values are static literals.
 */
struct Test519 : public HttpAotTest<Test519, 519> {
    static constexpr const char *DESCRIPTION = "BasicHTTP param encoding (W3C C.2 AOT Static)";
    using SM = SCE::Generated::test519::test519;
};

// Auto-register
inline static AotTestRegistrar<Test519> registrar_Test519;

}  // namespace SCE::W3C::AotTests
