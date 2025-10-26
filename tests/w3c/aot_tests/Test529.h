#pragma once
#include "SimpleAotTest.h"
#include "test529_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.5: Donedata content with integer literal
 *
 * Tests that integer literals in <donedata><content> are correctly passed
 * as _event.data to the completion event (done.state.{parentId}).
 *
 * Test flow:
 * 1. Initial state s0 with two child states: s01, s02
 * 2. Enter s01 → eventless transition → s02 (final state)
 * 3. s02 is final state with <donedata><content>21</content></donedata>
 * 4. DoneDataHelper evaluates content text "21" → converts to integer
 * 5. s02 completion triggers done.state.s0 event with _event.data = 21
 * 6. Transition in s0 with cond="_event.data == 21" → pass
 * 7. Fallback transition (no condition) → fail
 *
 * W3C SCXML 5.5 specifies that <content> without expr attribute uses
 * the text content as the event data. The implementation must evaluate
 * the text content (21) and pass it as _event.data to the completion event.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * ✅ All-or-Nothing Strategy:
 * - State machine structure: Fully static (compile-time states/transitions)
 * - ECMAScript expressions: Evaluated via JSEngine at runtime
 * - Content evaluation: DoneDataHelper evaluates text content "21" as integer
 * - No mixing of Interpreter/AOT engines (pure AOT with external JSEngine)
 *
 * ✅ Zero Duplication Principle:
 * - DoneDataHelper::evaluateContent() shared with Interpreter
 * - GuardHelper::evaluateGuard() shared condition evaluation (_event.data == 21)
 * - SystemVariableHelper::setupSystemVariables() shared _event binding
 *
 * W3C SCXML Features:
 * - W3C SCXML 5.5: Donedata content text evaluation
 * - W3C SCXML 3.8: Final state completion events (done.state.{id})
 * - W3C SCXML B.2: ECMAScript datamodel with integer literals
 * - W3C SCXML 3.12.1: Conditional transitions with _event.data access
 */
struct Test529 : public SimpleAotTest<Test529, 529> {
    static constexpr const char *DESCRIPTION = "Donedata content integer literal (W3C 5.5 AOT Static Hybrid)";
    using SM = RSM::Generated::test529::test529;
};

// Auto-register
inline static AotTestRegistrar<Test529> registrar_Test529;

}  // namespace RSM::W3C::AotTests
