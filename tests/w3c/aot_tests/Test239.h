#pragma once

#include "SimpleAotTest.h"
#include "test239_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 239
 *
 * Auto-generated AOT test registry.
 */
struct Test239 : public SimpleAotTest<Test239, 239> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 239 (AOT)";
    using SM = RSM::Generated::test239::test239;
};

inline static AotTestRegistrar<Test239> registrar_Test239;

}  // namespace RSM::W3C::AotTests
