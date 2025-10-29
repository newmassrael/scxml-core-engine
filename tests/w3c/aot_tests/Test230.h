#pragma once

#include "ScheduledAotTest.h"
#include "test230_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: Autoforward event field preservation
 *
 * Manual test verifying that autoforwarded events preserve all fields
 * (_event.name, _event.type, _event.sendid, _event.origin, _event.origintype,
 * _event.invokeid, _event.data) when sent from child to parent and back.
 *
 * Child process sends 'childToParent' event to parent with autoforward enabled.
 * Both parent and child log all _event fields to verify preservation.
 *
 * W3C SCXML 6.4: Invoke with autoforward (3s delayed send timeout)
 * W3C SCXML 6.2: Async event processing via runUntilCompletion()
 */
struct Test230 : public ScheduledAotTest<Test230, 230> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 6.4: Autoforward event fields (Static Hybrid AOT)";
    using SM = RSM::Generated::test230::test230;

    // Manual test: success is reaching Final state (no Pass/Fail distinction)
    static constexpr auto PASS_STATE = SM::State::Final;

    // W3C SCXML 6.2: Test uses 3s delayed send, so need longer timeout
    std::chrono::seconds getTimeout() const override {
        return std::chrono::seconds(5);
    }
};

inline static AotTestRegistrar<Test230> registrar_Test230;

}  // namespace RSM::W3C::AotTests
