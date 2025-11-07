#pragma once

#include "SimpleAotTest.h"
#include "test148_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Else clause execution with datamodel
 */
struct Test148 : public SimpleAotTest<Test148, 148> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.3: else clause execution when if/elseif both false";
    using SM = SCE::Generated::test148::test148;
};

// Auto-register
inline static AotTestRegistrar<Test148> registrar_Test148;

}  // namespace SCE::W3C::AotTests
