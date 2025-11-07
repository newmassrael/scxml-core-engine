#pragma once

#include "SimpleAotTest.h"
#include "test153_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach array iteration order (AOT JSEngine)
 */
struct Test153 : public SimpleAotTest<Test153, 153> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.6: foreach iteration order correctness";
    using SM = SCE::Generated::test153::test153;
};

// Auto-register
inline static AotTestRegistrar<Test153> registrar_Test153;

}  // namespace SCE::W3C::AotTests
