#pragma once
#include "SimpleAotTest.h"
#include "test521_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2.4/6.2.5: error.communication on invalid send target
 *
 * Tests that the processor raises error.communication when it cannot dispatch an event.
 * Uses targetexpr="undefined" to create an invalid target, verifying proper error handling.
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions: SendHelper (for error detection and error.communication raising)
 *
 * W3C SCXML Features:
 * - W3C SCXML 6.2.4: Dynamic target resolution via targetexpr attribute
 * - W3C SCXML 6.2.5: Error event generation (error.communication) for dispatch failures
 * - W3C SCXML 5.10.3: Event queue priority (internal error events before external timeout)
 * - W3C SCXML B.2: ECMAScript datamodel with undefined value evaluation
 *
 * Test Behavior:
 * - Send event with targetexpr="undefined" (invalid target)
 * - Processor detects invalid target and raises error.communication
 * - Internal error event processed before external timeout event
 * - Transition to pass on error.communication, fail on timeout
 *
 * Static Hybrid Implementation:
 * - Static states: s0 (initial), pass, fail
 * - Static transitions: error.communication → pass, timeout → fail
 * - Runtime expression: targetexpr="undefined" evaluated by JSEngine
 * - SendHelper.isInvalidTarget() detects undefined target and raises error.communication
 */
struct Test521 : public SimpleAotTest<Test521, 521> {
    static constexpr const char *DESCRIPTION =
        "error.communication on invalid targetexpr (W3C 6.2.4/6.2.5 AOT Static Hybrid)";
    using SM = RSM::Generated::test521::test521;
};

// Auto-register
inline static AotTestRegistrar<Test521> registrar_Test521;

}  // namespace RSM::W3C::AotTests
