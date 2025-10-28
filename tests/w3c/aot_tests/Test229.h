#pragma once

#include "SimpleAotTest.h"
#include "test229_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 229
 *
 * Auto-generated AOT test registry.
 */
struct Test229 : public SimpleAotTest<Test229, 229> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 229 (AOT)";
    using SM = RSM::Generated::test229::test229;
};

inline static AotTestRegistrar<Test229> registrar_Test229;

}  // namespace RSM::W3C::AotTests
