#pragma once

#include "SimpleAotTest.h"
#include "test207_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 207
 *
 * Auto-generated AOT test registry.
 */
struct Test207 : public SimpleAotTest<Test207, 207> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 207 (AOT)";
    using SM = RSM::Generated::test207::test207;
};

inline static AotTestRegistrar<Test207> registrar_Test207;

}  // namespace RSM::W3C::AotTests
