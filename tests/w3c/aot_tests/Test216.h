#pragma once

#include "SimpleAotTest.h"
#include "test216_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 216
 *
 * Auto-generated AOT test registry.
 */
struct Test216 : public SimpleAotTest<Test216, 216> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 216 (AOT)";
    using SM = RSM::Generated::test216::test216;
};

inline static AotTestRegistrar<Test216> registrar_Test216;

}  // namespace RSM::W3C::AotTests
