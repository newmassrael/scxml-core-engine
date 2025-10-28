#pragma once

#include "SimpleAotTest.h"
#include "test378_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 378
 *
 * Auto-generated AOT test registry.
 */
struct Test378 : public SimpleAotTest<Test378, 378> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 378 (AOT)";
    using SM = RSM::Generated::test378::test378;
};

inline static AotTestRegistrar<Test378> registrar_Test378;

}  // namespace RSM::W3C::AotTests
