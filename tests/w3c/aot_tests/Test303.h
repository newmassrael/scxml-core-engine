#pragma once

#include "SimpleAotTest.h"
#include "test303_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 303
 *
 * Auto-generated AOT test registry.
 */
struct Test303 : public SimpleAotTest<Test303, 303> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 303 (AOT)";
    using SM = RSM::Generated::test303::test303;
};

inline static AotTestRegistrar<Test303> registrar_Test303;

}  // namespace RSM::W3C::AotTests
