#pragma once

#include "SimpleAotTest.h"
#include "test158_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief Executable content document order (AOT)
 */
struct Test158 : public SimpleAotTest<Test158, 158> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 3.12.1: executable content executes in document order";
    using SM = RSM::Generated::test158::test158;
};

// Auto-register
inline static AotTestRegistrar<Test158> registrar_Test158;

}  // namespace RSM::W3C::AotTests
