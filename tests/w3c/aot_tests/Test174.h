#pragma once

#include "SimpleAotTest.h"
#include "test174_sm.h"

namespace SCE::W3C::AotTests {

// clang-format off
/**
 * @brief W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'type'.
 */
// clang-format on

struct Test174 : public SimpleAotTest<Test174, 174> {
    // clang-format off
    static constexpr const char *DESCRIPTION = "W3C SCXML 6.2: If 'typexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'type'.";
    // clang-format on
    using SM = SCE::Generated::test174::test174;
};

// Auto-register
inline static AotTestRegistrar<Test174> registrar_Test174;

}  // namespace SCE::W3C::AotTests
