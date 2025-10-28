#pragma once

#include "SimpleAotTest.h"
#include "test235_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 235
 *
 * Auto-generated AOT test registry.
 */
struct Test235 : public SimpleAotTest<Test235, 235> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 235 (AOT)";
    using SM = RSM::Generated::test235::test235;
};

inline static AotTestRegistrar<Test235> registrar_Test235;

}  // namespace RSM::W3C::AotTests
