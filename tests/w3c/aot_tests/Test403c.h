#pragma once

#include "SimpleAotTest.h"
#include "test403c_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.13: Optimal enabled transition set with preemption
 *
 * Tests optimal transition set selection with transition preemption in
 * parallel states.
 */
struct Test403c : public SimpleAotTest<Test403c, 4031> {
    static constexpr const char *DESCRIPTION = "Optimal transition set - preemption (W3C 3.13 AOT Static Hybrid)";
    using SM = RSM::Generated::test403c::test403c;
};

inline static AotTestRegistrar<Test403c> registrar_Test403c;

}  // namespace RSM::W3C::AotTests
