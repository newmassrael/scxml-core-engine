#pragma once

#include "SimpleAotTest.h"
#include "test236_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Event ordering - childToParent before done.invoke
 *
 * Tests that child's onexit actions (sending childToParent) execute before
 * done.invoke event is delivered to parent. Verifies proper event ordering
 * during invoke cancellation.
 *
 * W3C SCXML 6.4: Invoke with inline content (2s timeout)
 * W3C SCXML 6.2: Async event processing via runUntilCompletion()
 */
struct Test236 : public ScheduledAotTest<Test236, 236> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: Event ordering childToParent before done.invoke (Static Hybrid AOT)";
    using SM = RSM::Generated::test236::test236;

    // W3C SCXML 6.2: Test uses 2s delayed send, so need longer timeout
    std::chrono::seconds getTimeout() const override {
        return std::chrono::seconds(5);
    }
};

inline static AotTestRegistrar<Test236> registrar_Test236;

}  // namespace RSM::W3C::AotTests
