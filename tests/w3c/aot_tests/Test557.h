#pragma once
#include "SimpleAotTest.h"
#include "test557_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML B.2: ECMAScript XML DOM assignment with inline and file content
 *
 * Verifies that ECMAScript datamodel correctly assigns XML content (both inline
 * and file-loaded) as DOM structures, and that DOM API methods work correctly
 * for accessing and querying XML data.
 * According to W3C SCXML B.2, when a variable is assigned XML content in ECMAScript
 * datamodel, it MUST be converted to a DOM structure accessible via standard DOM APIs.
 *
 * Test flow:
 * 1. State machine starts in s0
 * 2. Initialize datamodel:
 *    a. var1 = inline XML <books><book title="title1"/></books> (converted to DOM)
 *    b. var2 = external file test557.txt containing XML (loaded and converted to DOM)
 * 3. Eventless transition from s0 → s1 with guard checking DOM API:
 *    var1.getElementsByTagName('book')[0].getAttribute('title') == 'title1'
 * 4. In s1, eventless transition with guard checking file-loaded DOM:
 *    var2.getElementsByTagName('book')[0].getAttribute('title') == 'title2'
 * 5. If both guards pass → transition to pass
 * 6. If any guard fails → transition to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel and DOM operations
 * - Uses Helper functions: DataModelInitHelper (for XML content parsing and file loading)
 * - Static transition structure with JSEngine for guard expression evaluation
 *
 * W3C SCXML Features:
 * - ECMAScript datamodel (B.2)
 * - Inline XML content assignment (B.2)
 * - External XML file loading via src attribute (5.2.2)
 * - DOM API support (getElementsByTagName, getAttribute) (B.2)
 * - Eventless transitions with conditional guards (3.13)
 *
 * Implementation Details:
 * - DataModelInitHelper parses inline XML content and creates DOM structure
 * - DataModelInitHelper loads external XML file and creates DOM structure
 * - JSEngine evaluates guard expressions using DOM API method calls
 * - Static Hybrid: Static state enum + JSEngine for DOM operations and guard evaluation
 * - File loading: test557.txt must be available in build output directory
 */
struct Test557 : public SimpleAotTest<Test557, 557> {
    static constexpr const char *DESCRIPTION = "ECMAScript XML DOM assignment (W3C B.2 AOT Static Hybrid)";
    using SM = SCE::Generated::test557::test557;
};

// Auto-register
inline static AotTestRegistrar<Test557> registrar_Test557;

}  // namespace SCE::W3C::AotTests
