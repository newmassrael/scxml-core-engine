#pragma once
#include "ScheduledAotTest.h"
#include "test241_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4.3: invoke with namelist + param consistency validation
 *
 * Tests that namelist and param behave consistently when passing datamodel values to child.
 * Invoked child will return success if its Var1 is set to 1, failure otherwise.
 *
 * Test scenario:
 * - State s01 invokes child with namelist="Var1" (pass parent Var1=1 to child)
 * - Child transitions to pass if Var1==1, else fail → sends success/failure event to parent
 * - On success event, transition to s02 (namelist worked)
 * - On failure event, transition to s03 (namelist failed)
 * - State s02 invokes child with <param name="Var1" expr="1"/> (explicit param)
 *   - Must receive success (since s01 received success)
 *   - On success → pass (consistent behavior)
 *   - On failure → fail (inconsistent)
 * - State s03 invokes child with <param name="Var1" expr="1"/> (explicit param)
 *   - Must receive failure (since s01 received failure)
 *   - On failure → pass (consistent behavior)
 *   - On success → fail (inconsistent)
 * - Timeout after 2 seconds → fail
 *
 * Expected behavior: Either both invokes succeed (s01→s02→pass) or both fail (s01→s03→pass).
 * The test validates namelist and param have identical semantics.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions:
 *   - InvokeHelper: defer/cancel/execute pattern for invoke lifecycle (W3C SCXML 6.4)
 *   - NamelistHelper: namelist attribute evaluation (W3C SCXML 6.4.3)
 *   - ParamHelper: param element evaluation (W3C SCXML 6.4.3) - via DoneDataHelper infrastructure
 *   - SendSchedulingHelper: delayed send scheduling (W3C SCXML 6.2)
 *   - EventMetadataHelper: _event variable binding
 *   - GuardHelper: condition evaluation (Var1 == 1 in child)
 *   - DatamodelHelper: datamodel initialization
 *   - SendHelper: send to parent (#_parent target)
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4.3: Invoke with namelist attribute (pass parent datamodel to child)
 * - W3C SCXML 6.4.3: Invoke with param element (explicit parameter passing)
 * - W3C SCXML 6.4: Invoke with inline content (<content><scxml>...</scxml></content>)
 * - W3C SCXML 3.12.1: Guard conditions (cond="Var1 == 1")
 * - W3C SCXML 6.2: Delayed send with timeout
 * - W3C SCXML 5.9.1: send target="#_parent" (child-to-parent communication)
 * - W3C SCXML 5.2: Scoped datamodel (parent Var1 vs child Var1)
 *
 * Note: This test intentionally declares Var1 in both parent and child to verify
 * proper scoping and parameter passing. The code generator correctly handles this
 * by treating parent and child as separate scopes.
 */
struct Test241 : public ScheduledAotTest<Test241, 241> {
    static constexpr const char *DESCRIPTION = "invoke namelist + param consistency (W3C 6.4.3 AOT Static Hybrid)";
    using SM = SCE::Generated::test241::test241;
};

// Auto-register
inline static AotTestRegistrar<Test241> registrar_Test241;

}  // namespace SCE::W3C::AotTests
