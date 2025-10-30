#pragma once

#include "SimpleAotTest.h"
#include "test151_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief Foreach declares new variables (AOT JSEngine)
 */
struct Test151 : public SimpleAotTest<Test151, 151> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.6: foreach variable declaration persistence";
    using SM = RSM::Generated::test151::test151;
};

// Auto-register
inline static AotTestRegistrar<Test151> registrar_Test151;

}  // namespace RSM::W3C::AotTests
