#pragma once

#include "SimpleAotTest.h"
#include "test149_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Neither if nor elseif executes
 */
struct Test149 : public SimpleAotTest<Test149, 149> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.3: if/elseif/else - neither clause executes";
    using SM = SCE::Generated::test149::test149;
};

// Auto-register
inline static AotTestRegistrar<Test149> registrar_Test149;

}  // namespace SCE::W3C::AotTests
