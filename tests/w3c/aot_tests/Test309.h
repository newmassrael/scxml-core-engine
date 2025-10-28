#pragma once

#include "SimpleAotTest.h"
#include "test309_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 309
 *
 * Auto-generated AOT test registry.
 */
struct Test309 : public SimpleAotTest<Test309, 309> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 309 (AOT)";
    using SM = RSM::Generated::test309::test309;
};

inline static AotTestRegistrar<Test309> registrar_Test309;

}  // namespace RSM::W3C::AotTests
