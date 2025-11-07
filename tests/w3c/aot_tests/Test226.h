#pragma once

#include "SimpleAotTest.h"
#include "test226_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 226
 *
 * Auto-generated AOT test registry.
 */
struct Test226 : public SimpleAotTest<Test226, 226> {
    static constexpr const char *DESCRIPTION = "W3C SCXML test 226 (AOT)";
    using SM = SCE::Generated::test226::test226;
};

inline static AotTestRegistrar<Test226> registrar_Test226;

}  // namespace SCE::W3C::AotTests
