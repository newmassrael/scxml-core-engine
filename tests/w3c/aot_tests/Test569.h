#pragma once
#include "SimpleAotTest.h"
#include "test569_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 5.8: SCXML Event I/O Processor location field verification
 *
 * Verifies that the location field is accessible in the _ioprocessors system variable
 * for the SCXML Event I/O processor in ECMAScript datamodel. The test checks that
 * _ioprocessors['scxml'].location contains a valid URI identifying the processor's
 * address for receiving events.
 *
 * Test flow:
 * 1. State machine initializes to s0
 * 2. Transition from s0: Guard condition evaluates _ioprocessors['scxml'].location
 * 3. SystemVariableHelper sets _ioprocessors in JSEngine session
 * 4. JSEngine evaluates cond="_ioprocessors['scxml'].location" → truthy value expected
 * 5. If location field exists and is truthy → transition to pass
 * 6. If location field missing or falsy → transition to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and system variable access
 * - Uses Helper functions: SystemVariableHelper (_ioprocessors setup), GuardHelper (guard evaluation)
 *
 * W3C SCXML Features:
 * - _ioprocessors system variable (W3C SCXML 5.8)
 * - SCXML Event I/O Processor location field (W3C SCXML 5.8.1)
 * - Guard condition with ECMAScript expression (W3C SCXML 5.9)
 * - System variable access in ECMAScript datamodel (W3C SCXML Appendix B.2)
 */
struct Test569 : public SimpleAotTest<Test569, 569> {
    static constexpr const char *DESCRIPTION = "SCXML I/O processor location (W3C 5.8 AOT Static Hybrid)";
    using SM = RSM::Generated::test569::test569;
};

// Auto-register
inline static AotTestRegistrar<Test569> registrar_Test569;

}  // namespace RSM::W3C::AotTests
