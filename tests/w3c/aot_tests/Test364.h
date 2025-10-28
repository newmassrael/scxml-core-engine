#pragma once

#include "SimpleAotTest.h"
#include "test364_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 364
 *
 * Auto-generated AOT test registry.
 */
struct Test364 : public SimpleAotTest<Test364, 364> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 364 (AOT)";
    using SM = RSM::Generated::test364::test364;
};

inline static AotTestRegistrar<Test364> registrar_Test364;

}  // namespace RSM::W3C::AotTests
