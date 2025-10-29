#pragma once
#include "SimpleAotTest.h"
#include "test244_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3.2: Invoke inline content + namelist datamodel value passing
 *
 * Tests that datamodel values can be specified via namelist attribute in invoke.
 * Parent state machine has Var1=1, invoked child state machine has Var1=0.
 * The namelist="Var1" should pass parent's Var1 value to child's datamodel.
 *
 * Expected behavior:
 * - Parent state machine: Sets Var1 to 1
 * - Invoke with namelist="Var1" → binds parent's Var1 value to child's datamodel
 * - Child state machine: Checks Var1 value (should be 1 from parent)
 * - If Var1 == 1, child sends "success" to #_parent
 * - If Var1 != 1, child sends "failure" to #_parent
 *
 * Note: Test schema validation intentionally fails due to duplicate Var1 definitions,
 * but the runtime behavior should correctly pass the parent's Var1 value to the child.
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
 * - ECMAScript datamodel (Var1 variable)
 * - Namelist binding: NamelistHelper reads parent's Var1 from datamodel → binds to child
 * - Child condition evaluation: GuardHelper evaluates "Var1 == 1" in child's JSEngine session
 * - Child→parent event communication: SendHelper with #_parent target
 *
 * W3C SCXML Features:
 * - 6.3.2: Namelist attribute for data passing to invoked child
 * - 6.4.1: Inline <content> invoke with embedded SCXML
 * - 6.2.6: #_parent target for child→parent event communication
 * - 5.2.2: ECMAScript datamodel with variable binding
 *
 * Helper Functions (Zero Duplication with Interpreter):
 * - NamelistHelper: Reads parent datamodel values and binds to child session
 * - GuardHelper: Evaluates child's transition conditions in JSEngine
 * - SendHelper: Handles #_parent target event routing
 * - DatamodelHelper: Initializes child datamodel with parent-provided values
 */
struct Test244 : public SimpleAotTest<Test244, 244> {
    static constexpr const char *DESCRIPTION = "Invoke namelist datamodel passing (W3C 6.3.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test244::test244;
};

// Auto-register
inline static AotTestRegistrar<Test244> registrar_Test244;

}  // namespace RSM::W3C::AotTests
