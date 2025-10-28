#pragma once

#include "SimpleAotTest.h"
#include "test310_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 310
 *
 * Auto-generated AOT test registry.
 */
struct Test310 : public SimpleAotTest<Test310, 310> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 310 (AOT)";
    using SM = RSM::Generated::test310::test310;
};

inline static AotTestRegistrar<Test310> registrar_Test310;

}  // namespace RSM::W3C::AotTests
