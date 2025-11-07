#pragma once
#include "SimpleAotTest.h"
#include "test525_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 4.6: Foreach element shallow copy semantics (minimal test)
 *
 * Tests that <foreach> creates a shallow copy of the array for iteration.
 * The test initializes Var1=[1,2,3], then in each foreach iteration:
 * - Increments Var2 counter only (no array modification)
 *
 * Expected behavior:
 * - Foreach iterates exactly 3 times (based on shallow copy of [1,2,3])
 * - After foreach completes, Var2 should equal 3
 * - Transition with cond="Var2 == 3" leads to pass state
 *
 * This is the minimal/canonical version of test 460's shallow copy validation.
 * Test 460 modifies the array during iteration, while test 525 only increments
 * the counter, making it a cleaner test of basic shallow copy semantics.
 *
 * W3C SCXML 4.6 specifies that foreach must iterate over a shallow copy of
 * the array, ensuring that the iteration count is determined before the loop
 * begins and cannot be affected by modifications during iteration.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time states/transitions)
 * - ECMAScript expressions: Evaluated via JSEngine at runtime
 * - No mixing of Interpreter/AOT engines (pure AOT with external JSEngine)
 *
 * ✅ Zero Duplication Principle:
 * - ForeachHelper::executeForeachWithActions() shared with Interpreter
 * - AssignHelper::isValidLocation() shared validation logic
 * - GuardHelper::evaluateGuard() shared condition evaluation
 * - DataModelInitHelper::initializeVariable() shared initialization
 *
 * W3C SCXML Features:
 * - W3C SCXML 4.6: Foreach shallow copy semantics
 * - W3C SCXML 5.9.2: Assign location validation
 * - W3C SCXML B.2: ECMAScript datamodel with array literals
 * - W3C SCXML 3.12.1: Conditional transitions with ECMAScript guards
 */
struct Test525 : public SimpleAotTest<Test525, 525> {
    static constexpr const char *DESCRIPTION = "Foreach shallow copy minimal (W3C 4.6 AOT Static Hybrid)";
    using SM = SCE::Generated::test525::test525;
};

// Auto-register
inline static AotTestRegistrar<Test525> registrar_Test525;

}  // namespace SCE::W3C::AotTests
