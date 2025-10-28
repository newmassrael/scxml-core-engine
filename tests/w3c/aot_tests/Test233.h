#pragma once

#include "SimpleAotTest.h"
#include "test233_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 233
 *
 * Auto-generated AOT test registry.
 */
struct Test233 : public SimpleAotTest<Test233, 233> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 233 (AOT)";
    using SM = RSM::Generated::test233::test233;
};

inline static AotTestRegistrar<Test233> registrar_Test233;

}  // namespace RSM::W3C::AotTests
