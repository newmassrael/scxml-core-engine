#pragma once

#include "SimpleAotTest.h"
#include "test294_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML Test 294
 *
 * Auto-generated AOT test registry.
 */
struct Test294 : public SimpleAotTest<Test294, 294> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 294 (AOT)";
    using SM = RSM::Generated::test294::test294;
};

inline static AotTestRegistrar<Test294> registrar_Test294;

}  // namespace RSM::W3C::AotTests
