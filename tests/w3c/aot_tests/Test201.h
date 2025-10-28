#pragma once

#include "SimpleAotTest.h"
#include "test201_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 201
 *
 * Auto-generated AOT test registry.
 */
struct Test201 : public SimpleAotTest<Test201, 201> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 201 (AOT)";
    using SM = RSM::Generated::test201::test201;
};

inline static AotTestRegistrar<Test201> registrar_Test201;

}  // namespace RSM::W3C::AotTests
