#pragma once

#include "SimpleAotTest.h"
#include "test234_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 234
 *
 * Auto-generated AOT test registry.
 */
struct Test234 : public SimpleAotTest<Test234, 234> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 234 (AOT)";
    using SM = RSM::Generated::test234::test234;
};

inline static AotTestRegistrar<Test234> registrar_Test234;

}  // namespace RSM::W3C::AotTests
