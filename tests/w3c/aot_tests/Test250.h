#pragma once

#include "SimpleAotTest.h"
#include "test250_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3: Event cancellation (manual test)
 *
 * Manual test verifying that cancelled events are not delivered.
 * Sends delayed event with 2s delay, then cancels it immediately.
 *
 * W3C SCXML 6.3: Send cancellation with sendid
 * W3C SCXML 6.2: Async event processing via runUntilCompletion()
 */
struct Test250 : public ScheduledAotTest<Test250, 250> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 6.3: Event cancellation (Static Hybrid AOT)";
    using SM = RSM::Generated::test250::test250;

    // Manual test: success is reaching Final state (no Pass/Fail distinction)
    static constexpr auto PASS_STATE = SM::State::Final;
};

inline static AotTestRegistrar<Test250> registrar_Test250;

}  // namespace RSM::W3C::AotTests
