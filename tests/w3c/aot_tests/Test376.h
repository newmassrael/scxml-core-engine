#pragma once

#include "SimpleAotTest.h"
#include "test376_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 376
 *
 * Auto-generated AOT test registry.
 */
struct Test376 : public SimpleAotTest<Test376, 376> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 376 (AOT)";
    using SM = RSM::Generated::test376::test376;
};

inline static AotTestRegistrar<Test376> registrar_Test376;

}  // namespace RSM::W3C::AotTests
