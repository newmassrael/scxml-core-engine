#pragma once

#include "SimpleAotTest.h"
#include "test302_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 302
 *
 * Auto-generated AOT test registry.
 */
struct Test302 : public SimpleAotTest<Test302, 302> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 302 (AOT)";
    using SM = RSM::Generated::test302::test302;
};

inline static AotTestRegistrar<Test302> registrar_Test302;

}  // namespace RSM::W3C::AotTests
