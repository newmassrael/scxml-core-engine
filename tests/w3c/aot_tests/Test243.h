#pragma once
#include "ScheduledAotTest.h"
#include "test243_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke with inline content and param passing
 *
 * Tests that datamodel values can be specified by param when invoking inline content.
 * Child state machine receives param and sends success/failure events to parent.
 *
 * Test scenario:
 * - State s0 invokes child with inline <content> and <param name="Var1" expr="1"/>
 *   - Child receives Var1=1 via param
 *   - Child has datamodel <data id="Var1" expr="0"/> (initial value)
 *   - Param overrides initial value: Var1 becomes 1
 *   - If Var1==1 → send success to parent
 *   - If Var1!=1 → send failure to parent
 * - Parent transitions:
 *   - On success → pass (param worked correctly)
 *   - On timeout → fail (child didn't respond)
 *   - On * (wildcard) → fail (child sent failure, param didn't work)
 *
 * Expected behavior: Child receives Var1=1 via param, sends success to parent → pass.
 *
 * ARCHITECTURE.md Compliance - Pure Static Parent + Static Hybrid Child:
 * - Parent: Fully static (compile-time states/transitions, no JSEngine)
 * - Child: Static Hybrid (static structure with JSEngine for condition evaluation: Var1==1)
 * - **Code Generation Enhancement**: Parser auto-adds child→parent events to parent Event enum
 *   - Child sends: success, failure
 *   - Parent Event enum: Success, Failure, Timeout, Wildcard
 *   - Prevents compilation errors when wildcard transitions catch child events
 * - Uses Helper functions:
 *   - InvokeHelper: defer/cancel/execute pattern for invoke lifecycle (W3C SCXML 6.4)
 *   - SendSchedulingHelper: delayed send scheduling for timeout (W3C SCXML 6.2)
 *   - SendHelper: sendToParent for child→parent communication (W3C SCXML 6.2)
 *   - JSEngine (child only): Expression evaluation for Var1==1 condition
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4: Invoke with inline content (<content><scxml>...</scxml></content>)
 * - W3C SCXML 6.4.1: Param element for passing data to invoked child
 * - W3C SCXML 6.2: Send to parent (#_parent target)
 * - W3C SCXML 5.9.3: Wildcard event transitions (event="*")
 * - W3C SCXML 5.2: Data model initialization with param override
 * - W3C SCXML B.2: ECMAScript datamodel with conditional evaluation
 *
 * Note: Child state machine (test243_child0) is automatically extracted from inline content.
 * The child uses Static Hybrid approach (static structure + JSEngine for Var1==1 evaluation).
 */
struct Test243 : public ScheduledAotTest<Test243, 243> {
    static constexpr const char *DESCRIPTION =
        "invoke inline content + param with child→parent events (W3C 6.4 AOT Pure Static)";
    using SM = SCE::Generated::test243::test243;
};

// Auto-register
inline static AotTestRegistrar<Test243> registrar_Test243;

}  // namespace SCE::W3C::AotTests
