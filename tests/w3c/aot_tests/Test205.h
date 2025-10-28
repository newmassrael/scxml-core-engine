#pragma once

#include "SimpleAotTest.h"
#include "test205_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 205
 *
 * Auto-generated AOT test registry.
 */
struct Test205 : public SimpleAotTest<Test205, 205> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 205 (AOT)";
    using SM = RSM::Generated::test205::test205;
};

inline static AotTestRegistrar<Test205> registrar_Test205;

}  // namespace RSM::W3C::AotTests
