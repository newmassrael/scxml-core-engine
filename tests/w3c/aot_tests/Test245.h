#pragma once
#include "SimpleAotTest.h"
#include "test245_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3.2: Invoke namelist with non-existent variable handling
 *
 * Tests that namelist does not set variables that don't exist in the invoked child's datamodel.
 * Parent state machine has Var2=3, invoked child state machine does NOT have Var2 in datamodel.
 * The namelist="Var2" should NOT bind parent's Var2 to child because Var2 is not defined in child.
 *
 * Expected behavior:
 * - Parent state machine: Sets Var2 to 3
 * - Invoke with namelist="Var2" → attempts to bind parent's Var2 to child's datamodel
 * - Child state machine: Does NOT have Var2 in datamodel (no <data id="Var2">)
 * - W3C SCXML 6.3.2: namelist ONLY binds variables that exist in child's datamodel
 * - Child condition: typeof Var2 !== 'undefined' → should be false (Var2 remains unbound)
 * - If Var2 is unbound (undefined), child sends "success" to #_parent
 * - If Var2 is bound (defined), child sends "failure" to #_parent
 *
 * This test validates that the implementation correctly handles the case where:
 * 1. Parent specifies a variable in namelist
 * 2. Child datamodel does not declare that variable
 * 3. The variable should remain unbound in the child (not injected)
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * Static elements (compile-time):
 * - State machine structure (State enum: S0, Pass, Fail)
 * - Event types (Event enum: Timeout, Success, Failure)
 * - Transition logic (switch-case)
 * - Inline child SCXML content (compile-time constant)
 *
 * Dynamic elements (runtime with JSEngine):
 * - ECMAScript datamodel (Var2 variable in parent only)
 * - Namelist binding: NamelistHelper validates child datamodel before binding
 * - Child condition evaluation: GuardHelper evaluates "typeof Var2 !== 'undefined'" in child's JSEngine session
 * - Child→parent event communication: SendHelper with #_parent target
 *
 * W3C SCXML Features:
 * - 6.3.2: Namelist attribute - only binds to variables declared in child's datamodel
 * - 6.4.1: Inline <content> invoke with embedded SCXML
 * - 6.2.6: #_parent target for child→parent event communication
 * - 5.2.2: ECMAScript datamodel with typeof operator for undefined variable detection
 *
 * Helper Functions (Zero Duplication with Interpreter):
 * - NamelistHelper: Validates child datamodel declarations before binding parent values
 * - GuardHelper: Evaluates child's transition conditions with typeof operator in JSEngine
 * - SendHelper: Handles #_parent target event routing
 * - DatamodelHelper: Initializes child datamodel (does NOT inject undeclared variables)
 */
struct Test245 : public SimpleAotTest<Test245, 245> {
    static constexpr const char *DESCRIPTION = "Invoke namelist non-existent var (W3C 6.3.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test245::test245;
};

// Auto-register
inline static AotTestRegistrar<Test245> registrar_Test245;

}  // namespace SCE::W3C::AotTests
