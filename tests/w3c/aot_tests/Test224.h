#pragma once

#include "SimpleAotTest.h"
#include "test224_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 224
 *
 * Auto-generated AOT test registry.
 */
struct Test224 : public SimpleAotTest<Test224, 224> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 224 (AOT)";
    using SM = RSM::Generated::test224::test224;
};

inline static AotTestRegistrar<Test224> registrar_Test224;

}  // namespace RSM::W3C::AotTests
