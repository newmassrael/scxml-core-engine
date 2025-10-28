#pragma once

#include "SimpleAotTest.h"
#include "test228_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 228
 *
 * Auto-generated AOT test registry.
 */
struct Test228 : public SimpleAotTest<Test228, 228> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 228 (AOT)";
    using SM = RSM::Generated::test228::test228;
};

inline static AotTestRegistrar<Test228> registrar_Test228;

}  // namespace RSM::W3C::AotTests
