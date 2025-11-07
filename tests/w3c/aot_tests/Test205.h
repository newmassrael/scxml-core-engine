#pragma once

#include "SimpleAotTest.h"
#include "test205_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: The sending SCXML Interpreter MUST not alter the content of the send
 *
 * Verifies that param data sent via send element is not modified during transmission.
 * Test sends event with param aParam=1, then validates that _event.data.aParam == 1
 * at the receiving state, ensuring data integrity throughout the event transmission process.
 */
struct Test205 : public SimpleAotTest<Test205, 205> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: The sending SCXML Interpreter MUST not alter the content of the send";
    using SM = SCE::Generated::test205::test205;
};

inline static AotTestRegistrar<Test205> registrar_Test205;

}  // namespace SCE::W3C::AotTests
