#pragma once

#include "SimpleAotTest.h"
#include "test403b_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 3.13: Optimal enabled transition set (parallel regions)
 *
 * Tests optimal transition set selection in parallel states where multiple
 * transitions can execute concurrently in different regions.
 */
struct Test403b : public SimpleAotTest<Test403b, 4030> {
    static constexpr const char *DESCRIPTION = "Optimal transition set - parallel (W3C 3.13 AOT Static Hybrid)";
    using SM = RSM::Generated::test403b::test403b;
};

inline static AotTestRegistrar<Test403b> registrar_Test403b;

}  // namespace RSM::W3C::AotTests
