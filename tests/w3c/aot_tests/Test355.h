#pragma once

#include "SimpleAotTest.h"
#include "test355_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 355
 *
 * Auto-generated AOT test registry.
 */
struct Test355 : public SimpleAotTest<Test355, 355> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 355 (AOT)";
    using SM = RSM::Generated::test355::test355;
};

inline static AotTestRegistrar<Test355> registrar_Test355;

}  // namespace RSM::W3C::AotTests
