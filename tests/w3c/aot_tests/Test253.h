#pragma once
#include "SimpleAotTest.h"
#include "test253_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.1: SCXML Event I/O Processor bidirectional communication
 *
 * This test verifies that the SCXML Event I/O Processor works in both directions
 * for parent-child state machine communication via inline <content> invoke.
 *
 * Test Flow:
 * 1. Parent invokes inline child state machine
 * 2. Child sends "childRunning" to parent using #_parent target
 * 3. Parent checks _event.origintype is SCXML Event I/O Processor
 * 4. Parent sends "parentToChild" to child using #_foo target
 * 5. Child checks _event.origintype is SCXML Event I/O Processor
 * 6. Child sends "success" if origintype matches, "failure" otherwise
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and expression evaluation
 * - Uses Helper functions:
 *   - InvokeContentHelper: Inline child SCXML parsing and instantiation
 *   - SendHelper: Event routing with #_parent and #_foo targets
 *   - EventMetadataHelper: _event variable binding for origintype access
 *   - GuardHelper: Condition evaluation with Var1/Var2 comparisons
 *   - AssignHelper: Variable assignment for _event.origintype capture
 *
 * W3C SCXML Features:
 * - C.1: SCXML Event I/O Processor URL validation
 * - 6.4: Invoke with inline <content> child
 * - 6.2: Send with #_parent and #_invokeid targets
 * - 5.9.2: _event.origintype system variable
 * - 5.4: Assign with datamodel location and expr
 * - 3.12.1: Conditional transitions with ECMAScript expressions
 */
struct Test253 : public SimpleAotTest<Test253, 253> {
    static constexpr const char *DESCRIPTION =
        "SCXML Event I/O Processor bidirectional communication (W3C C.1 AOT Static Hybrid)";
    using SM = RSM::Generated::test253::test253;
};

// Auto-register
inline static AotTestRegistrar<Test253> registrar_Test253;

}  // namespace RSM::W3C::AotTests
