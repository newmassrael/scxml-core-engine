#pragma once

#include "SimpleAotTest.h"
#include "test159_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief Error in executable content stops subsequent elements (AOT)
 */
struct Test159 : public SimpleAotTest<Test159, 159> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 3.12.1: error causes subsequent elements to be skipped";
    using SM = RSM::Generated::test159::test159;
};

// Auto-register
inline static AotTestRegistrar<Test159> registrar_Test159;

}  // namespace RSM::W3C::AotTests
