#pragma once

#include "SimpleAotTest.h"
#include "test198_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 198
 *
 * Auto-generated AOT test registry.
 */
struct Test198 : public SimpleAotTest<Test198, 198> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 198 (AOT)";
    using SM = RSM::Generated::test198::test198;
};

inline static AotTestRegistrar<Test198> registrar_Test198;

}  // namespace RSM::W3C::AotTests
