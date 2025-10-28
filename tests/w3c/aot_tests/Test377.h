#pragma once

#include "SimpleAotTest.h"
#include "test377_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 377
 *
 * Auto-generated AOT test registry.
 */
struct Test377 : public SimpleAotTest<Test377, 377> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 377 (AOT)";
    using SM = RSM::Generated::test377::test377;
};

inline static AotTestRegistrar<Test377> registrar_Test377;

}  // namespace RSM::W3C::AotTests
