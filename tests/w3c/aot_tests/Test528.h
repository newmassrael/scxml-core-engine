#pragma once
#include "SimpleAotTest.h"
#include "test528_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 5.5: Donedata content expr with error.execution
 *
 * Tests that illegal ECMAScript expressions in <donedata><content expr>
 * trigger error.execution events and result in empty _event.data.
 *
 * Test flow:
 * 1. Initial state s0 with two child states: s01, s02
 * 2. Enter s01 → eventless transition → s02 (final state)
 * 3. s02 is final state with <donedata><content expr="undefined.invalidProperty"/></donedata>
 * 4. DoneDataHelper evaluates illegal expression → throws exception
 * 5. Exception handler raises error.execution event
 * 6. s02 completion triggers done.state.s0 event with empty _event.data
 * 7. Transition in s0 with event="error.execution" cond="_event.data == ''" → pass
 * 8. Fallback transition (no condition) → fail
 *
 * W3C SCXML 5.5 specifies that errors during <content> expr evaluation must
 * trigger error.execution events. The implementation must catch exceptions
 * during expression evaluation and ensure _event.data is empty on error.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time states/transitions)
 * - ECMAScript expressions: Evaluated via JSEngine at runtime
 * - Error handling: DoneDataHelper catches JSEngine exceptions → error.execution
 * - No mixing of Interpreter/AOT engines (pure AOT with external JSEngine)
 *
 * ✅ Zero Duplication Principle:
 * - DoneDataHelper::evaluateContent() shared with Interpreter (includes error handling)
 * - GuardHelper::evaluateGuard() shared condition evaluation
 * - SystemVariableHelper::setupSystemVariables() shared _event binding
 * - Error.execution event generation logic shared between engines
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.5: Donedata content expr error handling
 * - W3C SCXML 5.10.1: Error.execution event on evaluation failures
 * - W3C SCXML 3.8: Final state completion events (done.state.{id})
 * - W3C SCXML B.2: ECMAScript datamodel with illegal property access
 * - W3C SCXML 3.12.1: Conditional transitions with _event.data validation
 */
struct Test528 : public SimpleAotTest<Test528, 528> {
    static constexpr const char *DESCRIPTION = "Donedata content expr error.execution (W3C 5.5 AOT Static Hybrid)";
    using SM = SCE::Generated::test528::test528;
};

// Auto-register
inline static AotTestRegistrar<Test528> registrar_Test528;

}  // namespace SCE::W3C::AotTests
