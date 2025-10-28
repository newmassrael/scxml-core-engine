#pragma once

#include "SimpleAotTest.h"
#include "test252_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 252
 *
 * Auto-generated AOT test registry.
 */
struct Test252 : public SimpleAotTest<Test252, 252> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 252 (AOT)";
    using SM = RSM::Generated::test252::test252;
};

inline static AotTestRegistrar<Test252> registrar_Test252;

}  // namespace RSM::W3C::AotTests
