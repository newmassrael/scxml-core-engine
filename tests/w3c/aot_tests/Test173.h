#pragma once

#include "SimpleAotTest.h"
#include "test173_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: targetexpr evaluates current datamodel value at send execution time
 */
struct Test173 : public SimpleAotTest<Test173, 173> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: targetexpr evaluates current datamodel value at send execution time";
    using SM = SCE::Generated::test173::test173;
};

// Auto-register
inline static AotTestRegistrar<Test173> registrar_Test173;

}  // namespace SCE::W3C::AotTests
