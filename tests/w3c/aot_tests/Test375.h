#pragma once

#include "SimpleAotTest.h"
#include "test375_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 375
 *
 * Auto-generated AOT test registry.
 */
struct Test375 : public SimpleAotTest<Test375, 375> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 375 (AOT)";
    using SM = RSM::Generated::test375::test375;
};

inline static AotTestRegistrar<Test375> registrar_Test375;

}  // namespace RSM::W3C::AotTests
