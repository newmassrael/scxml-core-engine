#pragma once

#include "SimpleAotTest.h"
#include "test242_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 242
 *
 * Auto-generated AOT test registry.
 */
struct Test242 : public SimpleAotTest<Test242, 242> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 242 (AOT)";
    using SM = RSM::Generated::test242::test242;
};

inline static AotTestRegistrar<Test242> registrar_Test242;

}  // namespace RSM::W3C::AotTests
