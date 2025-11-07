#pragma once

#include "SimpleAotTest.h"
#include "test298_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 298
 *
 * Auto-generated AOT test registry.
 */
struct Test298 : public SimpleAotTest<Test298, 298> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 298 (AOT)";
    using SM = SCE::Generated::test298::test298;
};

inline static AotTestRegistrar<Test298> registrar_Test298;

}  // namespace SCE::W3C::AotTests
