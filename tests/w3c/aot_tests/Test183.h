#pragma once

#include "SimpleAotTest.h"
#include "test183_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent send
 * element is evaluated and store it in this location
 */
struct Test183 : public SimpleAotTest<Test183, 183> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: If 'idlocation' is present, the SCXML Processor MUST generate an id when the parent send "
        "element is evaluated and store it in this location";
    using SM = SCE::Generated::test183::test183;
};

// Auto-register
inline static AotTestRegistrar<Test183> registrar_Test183;

}  // namespace SCE::W3C::AotTests
