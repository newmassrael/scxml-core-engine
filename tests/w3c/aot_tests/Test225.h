#pragma once

#include "SimpleAotTest.h"
#include "test225_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 225
 *
 * Auto-generated AOT test registry.
 */
struct Test225 : public SimpleAotTest<Test225, 225> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 225 (AOT)";
    using SM = SCE::Generated::test225::test225;
};

inline static AotTestRegistrar<Test225> registrar_Test225;

}  // namespace SCE::W3C::AotTests
