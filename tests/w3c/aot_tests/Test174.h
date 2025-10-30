#pragma once

#include "SimpleAotTest.h"
#include "test174_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is
 * evaluated and treat the result as if it had been entered as the value of 'type'.
 */
struct Test174 : public SimpleAotTest<Test174, 174> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is "
        "evaluated and treat the result as if it had been entered as the value of 'type'.";
    using SM = RSM::Generated::test174::test174;
};

// Auto-register
inline static AotTestRegistrar<Test174> registrar_Test174;

}  // namespace RSM::W3C::AotTests
