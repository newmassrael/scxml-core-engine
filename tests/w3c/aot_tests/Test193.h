#pragma once

#include "SimpleAotTest.h"
#include "test193_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.1: [When using the scxml event i/o processor] If neither the 'target' nor the 'targetexpr'
 * attribute is specified, the SCXML Processor MUST add the event to the external event queue of the sending session.
 */
struct Test193 : public SimpleAotTest<Test193, 193> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML C.1: [When using the scxml event i/o processor] If neither the 'target' nor the 'targetexpr' "
        "attribute is specified, the SCXML Processor MUST add the event to the external event queue of the sending "
        "session.";
    using SM = RSM::Generated::test193::test193;
};

// Auto-register
inline static AotTestRegistrar<Test193> registrar_Test193;

}  // namespace RSM::W3C::AotTests
