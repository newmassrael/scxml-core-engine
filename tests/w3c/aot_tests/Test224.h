#pragma once
#include "ScheduledAotTest.h"
#include "test224_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3.1: Invoke with idlocation attribute
 *
 * Tests that the automatically generated invoke ID follows the "stateid.platformid.index" format
 * and is correctly stored in the idlocation variable. The .index suffix ensures uniqueness when
 * multiple invokes exist in the same state.
 * and is correctly stored in the idlocation variable.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions: InvokeHelper, DatamodelHelper, GuardHelper
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.3.1: <invoke idlocation="Var1"> stores auto-generated ID
 * - W3C SCXML 6.2.1: <content> for inline SCXML child definition
 * - W3C SCXML 5.9.2: ECMAScript expression evaluation (Var1.indexOf(Var2))
 * - W3C SCXML 3.12.1: Automatic invoke ID generation follows "stateid.platformid" format
 *
 * Test Flow:
 * 1. Enter s0, schedule timeout, defer invoke (idlocation="Var1")
 * 2. Invoke is processed at macrostep end, child SCXML instantiated
 * 3. Child immediately reaches final state, raises done.invoke
 * 4. Parent transitions to s1 on any event (done.invoke or timeout)
 * 5. Check if Var1 starts with "s0." (indexOf check via JSEngine)
 * 6. Pass if ID format correct, fail otherwise
 *
 * Uses ScheduledAotTest for runUntilCompletion() to process:
 * - Deferred invoke execution (W3C SCXML 6.4)
 * - Child state machine lifecycle
 * - Event scheduler polling for timeout
 */
struct Test224 : public ScheduledAotTest<Test224, 224> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: When the platform generates an identifier for 'idlocation', the identifier MUST have the form "
        "stateid.platformid, where stateid is the id of the state containing this element and platformid is "
        "automatically generated.";
    using SM = SCE::Generated::test224::test224;
};

// Auto-register
inline static AotTestRegistrar<Test224> registrar_Test224;

}  // namespace SCE::W3C::AotTests
