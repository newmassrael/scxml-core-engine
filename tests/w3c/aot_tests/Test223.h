#pragma once

#include "SimpleAotTest.h"
#include "test223_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke idlocation attribute
 *
 * If the 'idlocation' attribute is present, the SCXML Processor MUST generate
 * an id automatically when the invoke element is evaluated and store it in the
 * location specified by 'idlocation'.
 */
struct Test223 : public SimpleAotTest<Test223, 223> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: If the 'idlocation' attribute is present, the SCXML Processor MUST generate an id "
        "automatically when the invoke element is evaluated and store it in the location specified by 'idlocation'.";
    using SM = RSM::Generated::test223::test223;
};

inline static AotTestRegistrar<Test223> registrar_Test223;

}  // namespace RSM::W3C::AotTests
