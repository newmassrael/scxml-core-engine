#pragma once

#include "SimpleAotTest.h"
#include "test150_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach with dynamic variables (AOT JSEngine)
 */
struct Test150 : public SimpleAotTest<Test150, 150> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.6: foreach declares new variable if item doesn't exist";
    using SM = SCE::Generated::test150::test150;
};

// Auto-register
inline static AotTestRegistrar<Test150> registrar_Test150;

}  // namespace SCE::W3C::AotTests
