#pragma once
#include "SimpleAotTest.h"
#include "test576_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.13: Parallel initial state with space-separated state IDs
 *
 * Tests that the 'initial' attribute of scxml with space-separated state IDs
 * (e.g., initial="s11p112 s11p122") correctly enters multiple deeply nested
 * parallel sibling states simultaneously.
 *
 * The test verifies:
 * - Both s11p112 and s11p122 are entered (non-default parallel siblings)
 * - s11p112 raises In-s11p112 event on entry
 * - s11p122 transitions on In-s11p112 to pass state
 * - Timeout transitions to fail if parallel entry fails
 *
 * ARCHITECTURE.md Compliance - Pure Static Approach:
 * - Fully static state machine (compile-time states/transitions)
 * - No JSEngine needed (all values are static literals)
 * - Space-separated initial states handled by codegen's _apply_parallel_initial_overrides()
 * - Hierarchical entry logic enters both parallel regions simultaneously
 *
 * W3C SCXML Features:
 * - W3C SCXML 3.6: Initial state attribute parsing with space-separated IDs
 * - W3C SCXML 3.13: Parallel state initial entry semantics
 * - W3C SCXML 3.3: Hierarchical state entry from root to target
 * - W3C SCXML 3.4: Parallel state child region entry (all regions entered)
 * - W3C SCXML 3.8.1: <raise> element for internal event generation
 * - W3C SCXML 5.9: Event matching and processing in parallel regions
 * - W3C SCXML 6.2: <send> with delay attribute for timeout guard
 */
struct Test576 : public SimpleAotTest<Test576, 576> {
    static constexpr const char *DESCRIPTION = "Parallel initial with space-separated states (W3C 3.13 AOT)";
    using SM = RSM::Generated::test576::test576;
};

// Auto-register
inline static AotTestRegistrar<Test576> registrar_Test576;

}  // namespace RSM::W3C::AotTests
