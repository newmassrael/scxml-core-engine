#pragma once

#include "SimpleAotTest.h"
#include "test302_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 302
 *
 * Auto-generated AOT test registry.
 */
struct Test302 : public SimpleAotTest<Test302, 302> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 302 (AOT)";
    using SM = SCE::Generated::test302::test302;
};

inline static AotTestRegistrar<Test302> registrar_Test302;

}  // namespace SCE::W3C::AotTests
