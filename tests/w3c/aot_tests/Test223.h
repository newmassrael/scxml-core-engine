#pragma once

#include "SimpleAotTest.h"
#include "test223_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 223
 *
 * Auto-generated AOT test registry.
 */
struct Test223 : public SimpleAotTest<Test223, 223> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 223 (AOT)";
    using SM = RSM::Generated::test223::test223;
};

inline static AotTestRegistrar<Test223> registrar_Test223;

}  // namespace RSM::W3C::AotTests
