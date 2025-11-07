#pragma once

#include "ScheduledAotTest.h"
#include "test192_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.1: Parent-child communication via #_<invokeid> target
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
 * - C.1: SCXML Event I/O Processor with #_<invokeid> target
 * - C.1: #_parent target for child-to-parent communication
 * - C.1: #_<invokeid> target sends events to child session's external queue
 * - 6.4.1: done.invoke event on child completion
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication: Uses SendHelper for target routing (Single Source of Truth)
 * - All-or-Nothing: Pure Static AOT (no JSEngine, no Interpreter mixing)
 */
struct Test192 : public ScheduledAotTest<Test192, 192> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML C.1: #_<invokeid> target sends events to child session's external queue";
    using SM = SCE::Generated::test192::test192;

    std::chrono::seconds getTimeout() const override {
        return std::chrono::seconds(10);
    }
};

// Auto-register
inline static AotTestRegistrar<Test192> registrar_Test192;

}  // namespace SCE::W3C::AotTests
