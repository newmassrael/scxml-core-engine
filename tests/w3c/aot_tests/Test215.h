#pragma once

#include "SimpleAotTest.h"
#include "test215_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke typeexpr attribute evaluation
 *
 * If the typeexpr attribute is present, the SCXML Processor MUST evaluate it
 * when the parent invoke element is evaluated and treat the result as if it
 * had been entered as the value of 'type'.
 */
struct Test215 : public SimpleAotTest<Test215, 215> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: If the typeexpr attribute is present, the SCXML Processor MUST evaluate it when the parent "
        "invoke element is evaluated and treat the result as if it had been entered as the value of 'type'.";
    using SM = RSM::Generated::test215::test215;
};

inline static AotTestRegistrar<Test215> registrar_Test215;

}  // namespace RSM::W3C::AotTests
