#pragma once

#include "SimpleAotTest.h"
#include "test304_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 304
 *
 * Auto-generated AOT test registry.
 */
struct Test304 : public SimpleAotTest<Test304, 304> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 304 (AOT)";
    using SM = SCE::Generated::test304::test304;
};

inline static AotTestRegistrar<Test304> registrar_Test304;

}  // namespace SCE::W3C::AotTests
