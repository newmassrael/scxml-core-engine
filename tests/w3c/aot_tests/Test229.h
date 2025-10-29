#pragma once
#include "ScheduledAotTest.h"
#include "test229_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4.6: Autoforward - automatic event forwarding to invoked child
 *
 * Tests that autoforward="true" on <invoke> causes events received by parent
 * to be automatically forwarded to invoked child state machine.
 *
 * Test Flow:
 * 1. Parent invokes child with autoforward="true"
 * 2. Child sends "childToParent" to parent
 * 3. Parent receives event, forwards it back to child (due to autoforward)
 * 4. Child receives forwarded event, sends "eventReceived" to parent
 * 5. Parent transitions to pass state
 *
 * W3C SCXML 6.4.6: "If the autoforward attribute is set to true, the SCXML Processor
 * must send an exact copy of every external event it receives to the invoked process."
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 *
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all events are static literals)
 * - Uses Helper functions:
 *   - InvokeHelper: defer/cancel/execute pending invokes (W3C SCXML 6.4)
 *   - SendSchedulingHelper: schedule delayed send events (W3C SCXML 6.2)
 *   - EventDataHelper: handle empty event data
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.4: invoke with inline <content> (child SCXML embedded)
 * - W3C SCXML 6.4.6: autoforward attribute (automatic event forwarding)
 * - W3C SCXML 6.2: delayed send with timeout
 * - W3C SCXML 5.8: target="#_parent" (send to parent state machine)
 */
struct Test229 : public ScheduledAotTest<Test229, 229> {
    static constexpr const char *DESCRIPTION = "autoforward with inline content (W3C 6.4.6 AOT Pure Static)";
    using SM = RSM::Generated::test229::test229;
};

// Auto-register
inline static AotTestRegistrar<Test229> registrar_Test229;

}  // namespace RSM::W3C::AotTests
