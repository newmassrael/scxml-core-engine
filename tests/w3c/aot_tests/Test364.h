#pragma once
#include "SimpleAotTest.h"
#include "test364_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 3.6/3.4: Default initial states and parallel configurations
 *
 * Tests that default initial states are entered when a compound state is entered:
 * 1. Initial attribute: Tests initial="s11p112 s11p122" (parallel state multi-target)
 * 2. Initial element: Tests <initial><transition target="s21p112 s21p122"/></initial>
 * 3. First child in document order: Tests default behavior when no initial specified
 *
 * W3C SCXML 3.6: "If the 'initial' attribute is not specified, the SCXML Processor must use
 * the first child state in document order as the default initial state."
 *
 * W3C SCXML 3.4: "When a parallel state is entered, all of its child states are entered in
 * parallel. If a child is a compound state, its initial state is entered."
 *
 * Test flow:
 * - s1: initial="s11p112 s11p122" (parallel initial attribute)
 *   → s11p112 raises In-s11p112
 *   → s11p122 receives In-s11p112 → transitions to s2
 * - s2: <initial><transition target="s21p112 s21p122"/></initial> (parallel initial element)
 *   → s21p112 raises In-s21p112
 *   → s21p122 receives In-s21p112 → transitions to s3
 * - s3: no initial (defaults to first child s31 → s311 → s3111)
 *   → s3111 transitions to pass
 *
 * Success: Reach pass (all three initial state methods work correctly)
 * Failure: Reach fail or timeout (incorrect initial state selection)
 */
struct Test364 : public SimpleAotTest<Test364, 364> {
    static constexpr const char *DESCRIPTION = "Default initial states and parallel configurations (W3C 3.6/3.4 AOT)";
    using SM = SCE::Generated::test364::test364;
};

// Auto-register
inline static AotTestRegistrar<Test364> registrar_Test364;

}  // namespace SCE::W3C::AotTests
