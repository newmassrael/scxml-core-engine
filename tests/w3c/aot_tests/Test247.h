#pragma once

#include "SimpleAotTest.h"
#include "test247_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 247
 *
 * Auto-generated AOT test registry.
 */
struct Test247 : public SimpleAotTest<Test247, 247> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 247 (AOT)";
    using SM = SCE::Generated::test247::test247;
};

inline static AotTestRegistrar<Test247> registrar_Test247;

}  // namespace SCE::W3C::AotTests
