#pragma once

#include "SimpleAotTest.h"
#include "test199_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 199
 *
 * Auto-generated AOT test registry.
 */
struct Test199 : public SimpleAotTest<Test199, 199> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 199 (AOT)";
    using SM = RSM::Generated::test199::test199;
};

inline static AotTestRegistrar<Test199> registrar_Test199;

}  // namespace RSM::W3C::AotTests
