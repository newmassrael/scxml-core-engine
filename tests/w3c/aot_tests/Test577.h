#pragma once
#include "SimpleAotTest.h"
#include "test577_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: error.communication on BasicHTTP send with no target
 *
 * Tests that a <send> with type="BasicHTTP" but no target attribute raises
 * error.communication event and adds it to the internal event queue.
 *
 * The test verifies:
 * - <send> with type="BasicHTTP" and no target raises error.communication
 * - error.communication event is added to internal queue (not external)
 * - State machine transitions to pass state on error.communication
 * - Timeout transitions to fail if error.communication not received
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all values are static literals)
 * - Uses Helper functions: SendHelper (validates BasicHTTP target)
 * - Compile-time validation of send target requirements
 *
 * W3C SCXML Features:
 * - W3C SCXML C.2: BasicHTTP Event I/O Processor error handling
 * - W3C SCXML 6.2: <send> element with type attribute
 * - W3C SCXML 5.10: error.communication event for invalid send operations
 * - W3C SCXML 5.9: Internal event queue processing
 * - W3C SCXML 3.12: Event-based state transitions
 */
struct Test577 : public SimpleAotTest<Test577, 577> {
    static constexpr const char *DESCRIPTION = "error.communication on BasicHTTP send with no target (W3C C.2 AOT)";
    using SM = RSM::Generated::test577::test577;
};

// Auto-register
inline static AotTestRegistrar<Test577> registrar_Test577;

}  // namespace RSM::W3C::AotTests
