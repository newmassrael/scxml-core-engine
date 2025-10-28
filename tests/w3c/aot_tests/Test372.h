#pragma once

#include "SimpleAotTest.h"
#include "test372_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 372
 *
 * Auto-generated AOT test registry.
 */
struct Test372 : public SimpleAotTest<Test372, 372> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 372 (AOT)";
    using SM = RSM::Generated::test372::test372;
};

inline static AotTestRegistrar<Test372> registrar_Test372;

}  // namespace RSM::W3C::AotTests
