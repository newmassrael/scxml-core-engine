/**
 * @brief W3C SCXML 6.4: Inline content invoke with #_<invokeid> target
 *
 * Validates parent-to-child event routing via #_<invokeid> target format.
 * Tests bidirectional communication in statically invoked child state machines:
 * 1. Parent invokes child via inline <content>
 * 2. Child sends event to parent using target="#_parent"
 * 3. Parent receives event and sends response to child using target="#_<invokeid>"
 * 4. Child receives event and enters final state
 * 5. Parent receives done.invoke and enters Pass state
 *
 * ARCHITECTURE.md Compliance:
 * - Pure Static AOT: Both parent and child state machines are AOT-compiled
 * - Zero Duplication: Uses SendHelper for target routing (Single Source of Truth)
 * - All-or-Nothing: No engine mixing, complete static code generation
 *
 * W3C SCXML References:
 * - 6.4: <invoke> element with inline <content>
 * - 6.4: #_<invokeid> target for parent-to-child communication
 * - 6.4.1: done.invoke event on child completion
 */
#include "aot_tests/Test192.h"
