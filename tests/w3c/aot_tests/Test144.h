#pragma once

#include "SimpleAotTest.h"
#include "test144_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 3.5.1: Event queue ordering
 *
 * Tests that events are processed in document order.
 */
struct Test144 : public SimpleAotTest<Test144, 144> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 4.2: raise element inserts events at rear of internal queue (FIFO ordering)";
    using SM = SCE::Generated::test144::test144;
};

// Auto-register
inline static AotTestRegistrar<Test144> registrar_Test144;

}  // namespace SCE::W3C::AotTests
