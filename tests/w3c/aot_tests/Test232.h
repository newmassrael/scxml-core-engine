#pragma once

#include "SimpleAotTest.h"
#include "test232_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 232
 *
 * Auto-generated AOT test registry.
 */
struct Test232 : public SimpleAotTest<Test232, 232> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 232 (AOT)";
    using SM = SCE::Generated::test232::test232;
};

inline static AotTestRegistrar<Test232> registrar_Test232;

}  // namespace SCE::W3C::AotTests
