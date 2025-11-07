#pragma once
#include "ScheduledAotTest.h"
#include "test240_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke with namelist and param
 *
 * Tests that datamodel values can be specified both by 'namelist' and by '<param>'.
 * Invoked child will return success if its Var1 is set to 1, failure otherwise.
 *
 * Test scenario:
 * - State s01 invokes child with namelist="Var1" (pass parent Var1=1 to child)
 * - Child transitions to pass if Var1==1, else fail
 * - On success event, transition to s02
 * - State s02 invokes child with <param name="Var1" expr="1"/> (explicit param)
 * - Child transitions to pass if Var1==1, else fail
 * - On success event, transition to pass
 * - Timeout after 2 seconds → fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions:
 *   - InvokeHelper: src file loading + inline content + namelist/param handling
 *   - NamelistHelper: namelist attribute evaluation (W3C SCXML 6.4.3)
 *   - ParamHelper: param element evaluation (W3C SCXML 6.4.3)
 *   - SendSchedulingHelper: delayed send scheduling (W3C SCXML 6.2)
 *   - EventMetadataHelper: _event variable binding
 *   - GuardHelper: condition evaluation (Var1 == 1 in child)
 *   - DatamodelHelper: datamodel initialization
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4.3: Invoke with namelist attribute (pass parent datamodel to child)
 * - W3C SCXML 6.4.3: Invoke with param element (explicit parameter passing)
 * - W3C SCXML 6.4: Invoke with inline content (<content><scxml>...</scxml></content>)
 * - W3C SCXML 3.12.1: Guard conditions (cond="Var1 == 1")
 * - W3C SCXML 6.2: Delayed send with timeout
 * - W3C SCXML 5.9.1: send target="#_parent" (child-to-parent communication)
 */
struct Test240 : public ScheduledAotTest<Test240, 240> {
    static constexpr const char *DESCRIPTION = "invoke namelist + param (W3C 6.4 AOT Static Hybrid)";
    using SM = SCE::Generated::test240::test240;
};

// Auto-register
inline static AotTestRegistrar<Test240> registrar_Test240;

}  // namespace SCE::W3C::AotTests
