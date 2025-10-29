#pragma once

#include "ScheduledAotTest.h"
#include "test192_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Inline content invoke with parent-child bidirectional communication
 *
 * This test validates the complete parent-child invoke infrastructure:
 * 1. Parent invokes child state machine via inline <content>
 * 2. Child sends event to parent using target="#_parent"
 * 3. Parent receives child event and sends response
 * 4. Parent sends event to child using target="#_<invokeid>"
 * 5. Child receives parent event and enters final state
 * 6. Parent receives done.invoke.invokedChild and enters Pass state
 *
 * Key W3C SCXML Features:
 * - 6.4: <invoke> element with inline content
 * - 6.4: #_parent target for child-to-parent communication
 * - 6.4: #_<invokeid> target for parent-to-child communication
 * - 6.4.1: done.invoke event on child completion
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Uses SendHelper for target routing (Single Source of Truth)
 * - All-or-Nothing: Pure Static AOT (no JSEngine, no Interpreter mixing)
 */
struct Test192 : public ScheduledAotTest<Test192, 192> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: Inline content invoke with #_<invokeid> target (Pure Static AOT)";
    using SM = RSM::Generated::test192::test192;

    std::chrono::seconds getTimeout() const override {
        return std::chrono::seconds(10);
    }
};

// Auto-register
inline static AotTestRegistrar<Test192> registrar_Test192;

}  // namespace RSM::W3C::AotTests
