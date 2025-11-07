#pragma once
#include "SimpleAotTest.h"
#include "test527_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.5: Donedata content expr evaluation
 *
 * Tests that the 'expr' attribute on <content> element within <donedata>
 * correctly evaluates ECMAScript expressions and passes the result as
 * _event.data to the completion event (done.state.{parentId}).
 *
 * Test flow:
 * 1. Initial state s0 with two child states: s01, s02
 * 2. Enter s01 → eventless transition → s02 (final state)
 * 3. s02 is final state with <donedata><content expr="'foo'"/></donedata>
 * 4. s02 completion triggers done.state.s0 event with _event.data = "foo"
 * 5. Transition in s0 with cond="_event.data == 'foo'" → pass
 * 6. Fallback transition (no condition) → fail
 *
 * W3C SCXML 5.5 specifies that <content> with expr attribute must evaluate
 * the expression and use the result as the event data. The _event.data field
 * in the completion event (done.state.{stateId}) must contain this value.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time states/transitions)
 * - ECMAScript expressions: Evaluated via JSEngine at runtime
 * - No mixing of Interpreter/AOT engines (pure AOT with external JSEngine)
 *
 * ✅ Zero Duplication Principle:
 * - DoneDataHelper::evaluateContent() shared with Interpreter
 * - GuardHelper::evaluateGuard() shared condition evaluation
 * - SystemVariableHelper::setupSystemVariables() shared _event binding
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.5: Donedata content expr attribute evaluation
 * - W3C SCXML 3.8: Final state completion events (done.state.{id})
 * - W3C SCXML B.2: ECMAScript datamodel with string literals
 * - W3C SCXML 3.12.1: Conditional transitions with _event.data access
 */
struct Test527 : public SimpleAotTest<Test527, 527> {
    static constexpr const char *DESCRIPTION = "Donedata content expr (W3C 5.5 AOT Static Hybrid)";
    using SM = SCE::Generated::test527::test527;
};

// Auto-register
inline static AotTestRegistrar<Test527> registrar_Test527;

}  // namespace SCE::W3C::AotTests
