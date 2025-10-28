#pragma once

#include "SimpleAotTest.h"
#include "test192_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 192
 *
 * Auto-generated AOT test registry.
 */
struct Test192 : public SimpleAotTest<Test192, 192> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 192 (AOT)";
    using SM = RSM::Generated::test192::test192;
};

inline static AotTestRegistrar<Test192> registrar_Test192;

}  // namespace RSM::W3C::AotTests
