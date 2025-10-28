#pragma once

#include "SimpleAotTest.h"
#include "test237_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 237
 *
 * Auto-generated AOT test registry.
 */
struct Test237 : public SimpleAotTest<Test237, 237> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 237 (AOT)";
    using SM = RSM::Generated::test237::test237;
};

inline static AotTestRegistrar<Test237> registrar_Test237;

}  // namespace RSM::W3C::AotTests
