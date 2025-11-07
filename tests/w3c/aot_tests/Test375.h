#pragma once

#include "SimpleAotTest.h"
#include "test375_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 375
 *
 * Auto-generated AOT test registry.
 */
struct Test375 : public SimpleAotTest<Test375, 375> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 375 (AOT)";
    using SM = SCE::Generated::test375::test375;
};

inline static AotTestRegistrar<Test375> registrar_Test375;

}  // namespace SCE::W3C::AotTests
