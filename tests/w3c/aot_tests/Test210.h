#pragma once

#include "SimpleAotTest.h"
#include "test210_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 210
 *
 * Auto-generated AOT test registry.
 */
struct Test210 : public SimpleAotTest<Test210, 210> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 210 (AOT)";
    using SM = RSM::Generated::test210::test210;
};

inline static AotTestRegistrar<Test210> registrar_Test210;

}  // namespace RSM::W3C::AotTests
