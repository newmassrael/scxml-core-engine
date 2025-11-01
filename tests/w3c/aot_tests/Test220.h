#pragma once

#include "SimpleAotTest.h"
#include "test220_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.4: invoke type attribute support
 *
 * Platforms MUST support http://www.w3.org/TR/scxml/, as a value for the
 * 'type' attribute.
 */
struct Test220 : public SimpleAotTest<Test220, 220> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.4: Platforms MUST support http://www.w3.org/TR/scxml/, as a value for the 'type' attribute";
    using SM = RSM::Generated::test220::test220;
};

inline static AotTestRegistrar<Test220> registrar_Test220;

}  // namespace RSM::W3C::AotTests
