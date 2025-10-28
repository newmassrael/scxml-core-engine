#pragma once

#include "SimpleAotTest.h"
#include "test220_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 220
 *
 * Auto-generated AOT test registry.
 */
struct Test220 : public SimpleAotTest<Test220, 220> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 220 (AOT)";
    using SM = RSM::Generated::test220::test220;
};

inline static AotTestRegistrar<Test220> registrar_Test220;

}  // namespace RSM::W3C::AotTests
