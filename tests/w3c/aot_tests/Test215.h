#pragma once

#include "SimpleAotTest.h"
#include "test215_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 215
 *
 * Auto-generated AOT test registry.
 */
struct Test215 : public SimpleAotTest<Test215, 215> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 215 (AOT)";
    using SM = RSM::Generated::test215::test215;
};

inline static AotTestRegistrar<Test215> registrar_Test215;

}  // namespace RSM::W3C::AotTests
